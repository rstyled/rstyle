//! Complete substituted receiver proof inputs, paired with unchanged observations.
use super::{Context, HirFnFacts, Lens, TypeObservation, diagnostic_support};
use rstyle_pdg::{ByteSpan, HirRef, data_receivers as records};
use rustc_hir::{self as hir, ExprKind};
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability, DerefAdjustKind};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Method {
    original: records::Method,
    concrete: Concrete,
}

#[derive(Serialize)]
struct Concrete {
    call: HirRef,
    source_span: ByteSpan,
    receiver: HirRef,
    receiver_type: TypeObservation,
    adjusted_type: TypeObservation,
    adjustments: Vec<Adjustment>,
    signature_inputs: Vec<TypeObservation>,
    signature_result: TypeObservation,
}

#[derive(Serialize)]
struct Adjustment {
    kind: records::Kind,
    target: TypeObservation,
}

pub(super) fn capture<'tcx>(
    context: &Context<'tcx>,
    lens: &Lens<'tcx>,
    facts: &HirFnFacts,
) -> Result<Vec<Method>, String> {
    let capture = facts.build.frontend_options.data::<records::Capture>()?;
    let [original] = capture.methods.as_slice() else {
        return Err("Shared-slice projection requires exactly one original method record".into());
    };
    if capture.identity != records::IDENTITY
        || rstyle_pdg::cpg_reader::projections::count_methods(&facts.body) != 1
    {
        return Err("Shared-slice original method inventory disagrees".into());
    }

    let id = diagnostic_support::hir_id(&original.call, "Projected method HIR is not local")?;
    if id.owner.def_id != lens.owner || context.typeck.hir_owner.def_id != lens.owner {
        return Err("Projected method and original typeck owners differ".into());
    }
    let hir::Node::Expr(expression) = context.tcx.hir_node(id) else {
        return Err("Projected method HIR is not an expression".into());
    };
    let concrete = observe(context, lens, expression, original)?;

    Ok(vec![Method {
        original: original.clone(),
        concrete,
    }])
}

/// A method call's receiver observations under the lens, before they are paired with the
/// original method record; an expression that is not a method call records only that.
pub(super) fn raw<'tcx>(
    context: &Context<'tcx>,
    lens: &Lens<'tcx>,
    expression: &hir::Expr<'tcx>,
) -> serde_json::Value {
    let ExprKind::MethodCall(_, receiver, _, _) = expression.kind else {
        return serde_json::json!({"method_call": false});
    };
    let identity = serde_json::json!({"method_call": true,
        "source_span": context.span(expression.span),
        "receiver": crate::hir_extraction::hir::reference(receiver.hir_id)});
    let Some(definition) = context.typeck.type_dependent_def_id(expression.hir_id) else {
        let mut record = identity;
        record["concrete"] =
            serde_json::json!({"error": "Projected method lacks its original resolved definition"});
        return record;
    };
    let bound = context
        .tcx
        .fn_sig(definition)
        .instantiate(context.tcx, context.typeck.node_args(expression.hir_id));
    let bound =
        match super::projection::normalize(context.tcx, lens.arguments, bound.skip_normalization())
        {
            Ok(bound) => bound,
            Err(error) => {
                let mut record = identity;
                record["concrete"] = serde_json::json!({"error": error.to_string()});
                return record;
            }
        };
    let signature = context.tcx.instantiate_bound_regions_with_erased(bound);
    let mut record = identity;
    record["concrete"] =
        serde_json::json!({"ok": receiver_observations(context, lens, receiver, signature)});

    record
}

/// A method call receiver's types, adjustments and signature under the lens.
fn receiver_observations<'tcx>(
    context: &Context<'tcx>,
    lens: &Lens<'tcx>,
    receiver: &hir::Expr<'tcx>,
    signature: rustc_middle::ty::FnSig<'tcx>,
) -> serde_json::Value {
    use super::record::fallible;
    let endpoint = |ty| lens.endpoint(context.tcx, ty);
    let adjustments: Vec<serde_json::Value> = context
        .typeck
        .expr_adjustments(receiver)
        .iter()
        .map(|adjustment| {
            let kind = match adjustment.kind {
                Adjust::Deref(DerefAdjustKind::Builtin) => "builtin_dereference",
                Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) => "shared_borrow",
                _ => "other",
            };
            serde_json::json!({"kind": kind, "target": fallible(endpoint(adjustment.target))})
        })
        .collect();
    let inputs = signature
        .inputs()
        .iter()
        .map(|ty| endpoint(*ty))
        .collect::<Result<Vec<_>, _>>();

    serde_json::json!({
        "receiver_type": fallible(endpoint(context.typeck.expr_ty(receiver))),
        "adjusted_type": fallible(endpoint(context.typeck.expr_ty_adjusted(receiver))),
        "adjustments": adjustments,
        "signature_inputs": fallible(inputs),
        "signature_result": fallible(endpoint(signature.output()))})
}

fn observe<'tcx>(
    context: &Context<'tcx>,
    lens: &Lens<'tcx>,
    expression: &hir::Expr<'tcx>,
    original: &records::Method,
) -> Result<Concrete, String> {
    let ExprKind::MethodCall(_, receiver, _, _) = expression.kind else {
        return Err("Projected original method HIR kind disagrees".into());
    };
    if original.source_span != context.span(expression.span)
        || original.receiver != crate::hir_extraction::hir::reference(receiver.hir_id)
    {
        return Err("Projected original method source/receiver identity disagrees".into());
    }

    let definition = context
        .typeck
        .type_dependent_def_id(expression.hir_id)
        .ok_or("Projected method lacks its original resolved definition")?;
    let bound = context
        .tcx
        .fn_sig(definition)
        .instantiate(context.tcx, context.typeck.node_args(expression.hir_id));
    let bound =
        super::projection::normalize(context.tcx, lens.arguments, bound.skip_normalization())
            .map_err(|error| error.to_string())?;
    // Observe the callable's value types without treating its late-bound
    // receiver region as an escaping free region or a lifetime proof.
    let signature = context.tcx.instantiate_bound_regions_with_erased(bound);
    let endpoint = |ty| lens.endpoint(context.tcx, ty);

    Ok(Concrete {
        call: original.call.clone(),
        source_span: original.source_span,
        receiver: original.receiver.clone(),
        receiver_type: endpoint(context.typeck.expr_ty(receiver))?,
        adjusted_type: endpoint(context.typeck.expr_ty_adjusted(receiver))?,
        adjustments: adjustments(context, lens, receiver, original)?,
        signature_inputs: signature
            .inputs()
            .iter()
            .map(|ty| endpoint(*ty))
            .collect::<Result<_, _>>()?,
        signature_result: endpoint(signature.output())?,
    })
}

fn adjustments<'tcx>(
    context: &Context<'tcx>,
    lens: &Lens<'tcx>,
    receiver: &hir::Expr<'tcx>,
    original: &records::Method,
) -> Result<Vec<Adjustment>, String> {
    let adjustments = context.typeck.expr_adjustments(receiver);
    if adjustments.len() != original.adjustments.len()
        || adjustments.len() > records::MAX_ADJUSTMENTS
    {
        return Err("Projected receiver adjustment inventory disagrees".into());
    }

    adjustments
        .iter()
        .zip(&original.adjustments)
        .map(|(adjustment, original)| {
            let agrees = matches!(
                (&adjustment.kind, &original.kind),
                (
                    Adjust::Deref(DerefAdjustKind::Builtin),
                    records::Kind::BuiltinDereference
                ) | (
                    Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)),
                    records::Kind::SharedBorrow
                )
            );
            if !agrees {
                return Err(
                    "Projected receiver requires original builtin/shared adjustment kinds".into(),
                );
            }

            Ok(Adjustment {
                kind: original.kind.clone(),
                target: lens.endpoint(context.tcx, adjustment.target)?,
            })
        })
        .collect()
}
