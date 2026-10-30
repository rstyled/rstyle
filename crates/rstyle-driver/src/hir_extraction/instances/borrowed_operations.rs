//! Selected concrete AsRef observations, without library, lifetime or origin authority.
use super::super::hir::{Context, reference};
use super::diagnostic_support::ArgumentsProjection;
use super::{
    comparisons,
    reference_owners::Projection,
    types::{self, endpoint},
};
use rustc_hir::{Expr, ExprKind, Mutability, def_id::DefId};
use rustc_middle::ty::{self, Ty, TyCtxt};
use serde::Serialize;

mod receiver;
mod signature;

#[derive(Serialize)]
pub(super) struct Observation {
    identity: &'static str,
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    expanded: bool,
    requested: comparisons::CompilerDefinition,
    requested_trait: comparisons::CompilerDefinition,
    original_arguments: types::Arguments,
    requested_arguments: types::Arguments,
    selected: comparisons::CompilerDefinition,
    selected_arguments: types::Arguments,
    selected_impl: comparisons::CompilerDefinition,
    selected_trait: comparisons::CompilerDefinition,
    requested_signature: signature::Signature,
    signature: signature::Signature,
    receiver_adjustments: receiver::Observation,
    receiver_type: types::Arguments,
    result_type: types::Arguments,
}

/// Only the exact core AsRef diagnostic trait is a candidate. Other operations
/// remain absent from this roster and require their own whole-body treatment.
pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    projection: &Projection<'tcx>,
) -> Result<Option<Observation>, String> {
    let ExprKind::MethodCall(_, receiver, operands, _) = expression.kind else {
        return Ok(None);
    };
    projection.node_ty(context.tcx, context.typeck, expression.hir_id)?;

    let Some(requested) = context.typeck.type_dependent_def_id(expression.hir_id) else {
        return Ok(None);
    };
    let Some(trait_id) = context.tcx.trait_of_assoc(requested) else {
        return Ok(None);
    };

    if !context
        .tcx
        .is_diagnostic_item(rustc_span::sym::AsRef, trait_id)
        || context.tcx.item_name(requested).as_str() != "as_ref"
    {
        return Ok(None);
    }

    if !operands.is_empty() || expression.span.from_expansion() {
        return Err(
            "AsRef observation requires one unexpanded receiver and no extra operands".into(),
        );
    }

    let selected = selection(context, expression, projection, requested)?;
    let receiver_type = projection.node_ty(context.tcx, context.typeck, receiver.hir_id)?;
    if !matches!(receiver_type.kind(), ty::Ref(_, inner, Mutability::Not) if haystack(context.tcx, *inner))
    {
        return Err("AsRef receiver is outside shared slice-u8/str instances".into());
    }
    Ok(Some(observation(
        context, expression, projection, &selected,
    )?))
}

struct Selection<'tcx> {
    requested: DefId,
    arguments: ty::GenericArgsRef<'tcx>,
    selected: ty::Instance<'tcx>,
    implementation: DefId,
    requested_trait: DefId,
    selected_trait: DefId,
}

fn selection<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    projection: &Projection<'tcx>,
    requested: DefId,
) -> Result<Selection<'tcx>, String> {
    let tcx = context.tcx;
    let original = context.typeck.node_args(expression.hir_id);
    let arguments = projection.args(tcx, original)?;
    let [self_arg, target_arg] = &arguments[..] else {
        return Err("AsRef requires its exact Self and target arguments".into());
    };

    if !matches!(self_arg.kind(), ty::GenericArgKind::Type(ty) if haystack(tcx, ty))
        || !matches!(target_arg.kind(), ty::GenericArgKind::Type(ty) if bytes(tcx, ty))
    {
        return Err("AsRef substitution is outside slice-u8/str to slice-u8".into());
    }

    let selected = projection.select(tcx, requested, original)?;
    let (implementation, requested_trait, selected_trait) =
        owners(tcx, requested, selected.def_id())?;
    Ok(Selection {
        requested,
        arguments,
        selected,
        implementation,
        requested_trait,
        selected_trait,
    })
}

fn owners(
    tcx: TyCtxt<'_>,
    requested: DefId,
    selected: DefId,
) -> Result<(DefId, DefId, DefId), String> {
    let implementation = tcx
        .impl_of_assoc(selected)
        .ok_or("AsRef selected method has no impl owner")?;
    let selected_trait = tcx
        .impl_opt_trait_id(implementation)
        .ok_or("AsRef selected owner is not a trait impl")?;
    let requested_trait = tcx
        .trait_of_assoc(requested)
        .ok_or("AsRef requested method has no trait owner")?;

    if selected_trait != requested_trait
        || [
            requested,
            requested_trait,
            selected,
            implementation,
            selected_trait,
        ]
        .iter()
        .any(|id| id.is_local())
    {
        return Err(
            "AsRef selected implementation lacks the external requested trait owner".into(),
        );
    }
    Ok((implementation, requested_trait, selected_trait))
}

fn observation<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    projection: &Projection<'tcx>,
    selected: &Selection<'tcx>,
) -> Result<Observation, String> {
    let tcx = context.tcx;
    let original = context.typeck.node_args(expression.hir_id);
    let ExprKind::MethodCall(_, receiver, _, _) = expression.kind else {
        unreachable!()
    };
    Ok(Observation {
        identity: "rstyle.rust.borrowed-as-ref-observation/1",
        hir: reference(expression.hir_id),
        span: context.span(expression.span),
        expanded: false,
        requested: comparisons::definition(tcx, selected.requested),
        requested_trait: comparisons::definition(tcx, selected.requested_trait),
        original_arguments: types::arguments(tcx, original),
        requested_arguments: types::arguments(tcx, selected.arguments),
        selected: comparisons::definition(tcx, selected.selected.def_id()),
        selected_arguments: types::arguments(tcx, selected.selected.args),
        selected_impl: comparisons::definition(tcx, selected.implementation),
        selected_trait: comparisons::definition(tcx, selected.selected_trait),
        requested_signature: signature::capture(tcx, selected.requested, selected.arguments)?,
        signature: signature::capture(tcx, selected.selected.def_id(), selected.selected.args)?,
        receiver_adjustments: receiver::capture(context, receiver, projection)?,
        receiver_type: endpoint(
            tcx,
            projection.node_ty(tcx, context.typeck, receiver.hir_id)?,
        )?,
        result_type: endpoint(
            tcx,
            projection.node_ty(tcx, context.typeck, expression.hir_id)?,
        )?,
    })
}

fn bytes<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> bool {
    value == Ty::new_slice(tcx, tcx.types.u8)
}

fn haystack<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> bool {
    value == tcx.types.str_ || bytes(tcx, value)
}
