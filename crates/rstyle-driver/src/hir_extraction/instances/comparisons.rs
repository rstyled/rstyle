//! Concrete comparison observations, never primitive-operation authority.
//!
//! The original overloaded dispatch remains intact. Independent source and
//! implementation proofs must authorize any later scalar interpretation.

use super::super::hir::{Context, reference};
use super::projection::Projection;
use super::{Definition, adjustments, definition_record, types};
use rustc_hir::{BinOpKind, Expr, ExprKind, def_id::DefId};
use rustc_middle::ty::{self, TyCtxt};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Observation {
    identity: &'static str,
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    expanded: bool,
    operator: &'static str,
    requested: CompilerDefinition,
    requested_trait: Option<CompilerDefinition>,
    original_arguments: types::Arguments,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Serialize)]
struct CompilerDefinition {
    #[serde(flatten)]
    definition: Definition,
    crate_hash: String,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Outcome {
    Resolved(Box<Selection>),
    Unresolved { reason: String },
    Unsupported { reason: String },
    Error { reason: String },
}

#[derive(Serialize)]
struct Selection {
    requested_arguments: types::Arguments,
    selected: CompilerDefinition,
    selected_arguments: types::Arguments,
    selected_impl: CompilerDefinition,
    selected_trait: CompilerDefinition,
    requested_signature: Signature,
    signature: Signature,
    operand_adjustments: [adjustments::Observation; 2],
    operand_types: types::Arguments,
    result_type: types::Arguments,
}

#[derive(Serialize)]
struct Signature {
    inputs: types::Arguments,
    output: types::Arguments,
}

/// Observe only compiler-overloaded comparisons in this owning body.
/// Builtin comparisons and other expressions return None; callers must retain
/// and independently check every original node, including unsupported operators.
pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    projection: &Projection<'tcx>,
) -> Result<Option<Observation>, String> {
    let ExprKind::Binary(operator, _, _) = expression.kind else {
        return Ok(None);
    };
    let Some(operator) = comparison(operator.node) else {
        return Ok(None);
    };
    // Check the owner before querying the TypeckResults local table.
    projection
        .node_ty(context.tcx, context.typeck, expression.hir_id)
        .map_err(|error| error.to_string())?;
    let Some(requested) = context.typeck.type_dependent_def_id(expression.hir_id) else {
        return Ok(None);
    };

    let arguments = context.typeck.node_args(expression.hir_id);
    let outcome = selection(context, expression, projection, requested).unwrap_or_else(failure);
    Ok(Some(Observation {
        identity: "rstyle.rust.primitive-comparison-observation/1",
        hir: reference(expression.hir_id),
        span: context.span(expression.span),
        expanded: expression.span.from_expansion(),
        operator,
        requested: definition(context.tcx, requested),
        requested_trait: context
            .tcx
            .trait_of_assoc(requested)
            .map(|id| definition(context.tcx, id)),
        original_arguments: types::arguments(context.tcx, arguments),
        outcome,
    }))
}

fn comparison(operator: BinOpKind) -> Option<&'static str> {
    match operator {
        BinOpKind::Eq => Some("=="),
        BinOpKind::Ne => Some("!="),
        BinOpKind::Lt => Some("<"),
        BinOpKind::Le => Some("<="),
        BinOpKind::Gt => Some(">"),
        BinOpKind::Ge => Some(">="),
        _ => None,
    }
}

fn selection<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    projection: &Projection<'tcx>,
    requested: DefId,
) -> Result<Outcome, super::projection::Error> {
    let tcx = context.tcx;
    let arguments = context.typeck.node_args(expression.hir_id);
    let substituted = projection.args(tcx, arguments)?;
    let instance = projection.select(tcx, requested, arguments)?;
    let Some(implementation) = tcx.impl_of_assoc(instance.def_id()) else {
        return Ok(Outcome::Unsupported {
            reason: "selected comparison has no implementation owner".into(),
        });
    };
    let Some(selected_trait) = tcx.impl_opt_trait_id(implementation) else {
        return Ok(Outcome::Unsupported {
            reason: "selected comparison owner is not a trait implementation".into(),
        });
    };

    let requested_signature = instantiated_signature(tcx, requested, substituted)?;
    let signature = instantiated_signature(tcx, instance.def_id(), instance.args)?;
    let ExprKind::Binary(_, left, right) = expression.kind else {
        unreachable!("selection only receives binary comparisons");
    };
    let operand_adjustments = match (
        adjustments::operand(context, left, projection),
        adjustments::operand(context, right, projection),
    ) {
        (Ok(left), Ok(right)) => [left, right],
        (Err(reason), _) | (_, Err(reason)) => return Ok(Outcome::Unsupported { reason }),
    };
    let operands = [
        projection.node_ty(tcx, context.typeck, left.hir_id)?,
        projection.node_ty(tcx, context.typeck, right.hir_id)?,
    ];
    let result = projection.node_ty(tcx, context.typeck, expression.hir_id)?;

    Ok(Outcome::Resolved(Box::new(Selection {
        requested_arguments: types::arguments(tcx, substituted),
        selected: definition(tcx, instance.def_id()),
        selected_arguments: types::arguments(tcx, instance.args),
        selected_impl: definition(tcx, implementation),
        selected_trait: definition(tcx, selected_trait),
        requested_signature,
        signature,
        operand_adjustments,
        operand_types: type_list(tcx, operands),
        result_type: type_list(tcx, [result]),
    })))
}

fn instantiated_signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    definition: DefId,
    arguments: ty::GenericArgsRef<'tcx>,
) -> Result<Signature, super::projection::Error> {
    let signature = tcx
        .try_normalize_erasing_regions(
            ty::TypingEnv::fully_monomorphized(),
            tcx.fn_sig(definition).instantiate(tcx, arguments),
        )
        .map_err(|_| super::projection::Error::Normalization)?
        .skip_binder();

    let inputs = type_list(tcx, signature.inputs().iter().copied());
    let output = type_list(tcx, [signature.output()]);
    if !inputs.complete() || !output.complete() {
        return Err(super::projection::Error::UnsupportedType);
    }
    Ok(Signature { inputs, output })
}

fn type_list<'tcx>(
    tcx: TyCtxt<'tcx>,
    values: impl IntoIterator<Item = ty::Ty<'tcx>>,
) -> types::Arguments {
    types::arguments(
        tcx,
        tcx.mk_args_from_iter(values.into_iter().map(ty::GenericArg::from)),
    )
}

fn definition(tcx: TyCtxt<'_>, id: DefId) -> CompilerDefinition {
    CompilerDefinition {
        definition: definition_record(tcx, id),
        crate_hash: tcx.crate_hash(id.krate).to_string(),
    }
}

fn failure(error: super::projection::Error) -> Outcome {
    use super::projection::Error;

    let reason = error.to_string();
    match error {
        Error::Unresolved | Error::NoInstance => Outcome::Unresolved { reason },
        Error::Normalization | Error::Resolution => Outcome::Error { reason },
        _ => Outcome::Unsupported { reason },
    }
}
