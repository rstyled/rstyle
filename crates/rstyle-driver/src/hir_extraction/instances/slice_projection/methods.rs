//! Complete substituted receiver proof inputs, paired with unchanged observations.
use super::{Context, HirFnFacts, HirNode, Lens, TypeObservation, diagnostic_support};
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
    if capture.identity != records::IDENTITY || count_methods(&facts.body) != 1 {
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

fn count_methods(node: &HirNode) -> usize {
    usize::from(node.kind == "expr_method_call")
        + node.children.iter().map(count_methods).sum::<usize>()
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
