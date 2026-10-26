//! Separately scoped immediate callback calls; never merge nested owners into root calls.
use super::*;

#[derive(Serialize)]
pub(super) struct Observation {
    scope: &'static str,
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    definition: Definition,
    capture_count: usize,
    callable: types::Arguments,
    calls: Vec<super::Observation>,
}

pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
) -> Option<Observation> {
    let ExprKind::Closure(closure) = expression.kind else {
        return None;
    };
    if !context.immediate_core_callback(
        expression,
        &[
            "core::option::Option::<T>::and_then",
            "std::option::Option::<T>::and_then",
        ],
    ) {
        return None;
    }
    let ty::Closure(definition, _) = context.typeck.expr_ty(expression).kind() else {
        return None;
    };
    if *definition != closure.def_id.to_def_id() {
        return None;
    }

    Some(Observation {
        scope: "immediate-core-option-and-then-callback-owning-body; nested owners excluded",
        hir: reference(expression.hir_id),
        span: context.span(expression.span),
        definition: definition_record(context.tcx, *definition),
        capture_count: context.tcx.closure_captures(closure.def_id).len(),
        callable: types::arguments(
            context.tcx,
            context
                .tcx
                .mk_args(&[context.typeck.expr_ty(expression).into()]),
        ),
        calls: callback_calls(context, closure),
    })
}

fn callback_calls<'tcx>(
    context: &Context<'tcx>,
    closure: &rustc_hir::Closure<'tcx>,
) -> Vec<super::Observation> {
    let callback = Context {
        typeck: context.tcx.typeck(closure.def_id),
        tcx: context.tcx,
        function_span: context.function_span,
        source_bytes: context.source_bytes.clone(),
        sources: context.sources.clone(),
        unit_context: context.unit_context.clone(),
        result: context.result,
        profile: context.profile,
    };
    let mut scan = Scan {
        context: &callback,
        calls: Vec::new(),
        callbacks: Vec::new(),
        observe_callbacks: false,
    };
    scan.visit_expr(context.tcx.hir_body(closure.body).value);

    scan.calls
}
