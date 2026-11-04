//! Complete substituted receiver proof inputs, paired with unchanged observations.
use super::{Context, Lens};
use rustc_hir::{self as hir, ExprKind};
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability, DerefAdjustKind};

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
