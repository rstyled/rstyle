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
    #[serde(skip_serializing_if = "Option::is_none")]
    complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<rstyle_pdg::HirRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_callback: Option<rstyle_pdg::HirRef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    callbacks: Vec<Observation>,
}

pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    parent_callback: Option<&rstyle_pdg::HirRef>,
) -> Option<Observation> {
    let ExprKind::Closure(closure) = expression.kind else {
        return None;
    };
    let result_reader = context.usize_reader_callback(expression);
    if !result_reader
        && !context.immediate_core_callback(
            expression,
            &[
                "core::option::Option::<T>::and_then",
                "std::option::Option::<T>::and_then",
            ],
        )
    {
        return None;
    }
    let ty::Closure(definition, _) = context.typeck.expr_ty(expression).kind() else {
        return None;
    };
    if *definition != closure.def_id.to_def_id() {
        return None;
    }

    Some(record(
        context,
        expression,
        Selection {
            result_reader,
            parent_callback,
        },
    ))
}

struct Selection<'a> {
    result_reader: bool,
    parent_callback: Option<&'a rstyle_pdg::HirRef>,
}

fn record<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    selection: Selection<'_>,
) -> Observation {
    let ExprKind::Closure(closure) = expression.kind else {
        unreachable!("validated callback expression")
    };
    let result_reader = selection.result_reader;
    let nested = callback_calls(context, expression, result_reader);

    Observation {
        scope: if result_reader {
            "immediate-core-result-reader-callback-owning-body; nested owners separately scoped"
        } else {
            "immediate-core-option-and-then-callback-owning-body; nested owners excluded"
        },
        hir: reference(expression.hir_id),
        span: context.span(expression.span),
        definition: definition_record(context.tcx, closure.def_id.to_def_id()),
        capture_count: context.tcx.closure_captures(closure.def_id).len(),
        callable: types::arguments(
            context.tcx,
            context
                .tcx
                .mk_args(&[context.typeck.expr_ty(expression).into()]),
        ),
        calls: nested.calls,
        complete: result_reader.then_some(true),
        body: result_reader.then(|| reference(context.tcx.hir_body(closure.body).value.hir_id)),
        parent_callback: if result_reader {
            selection.parent_callback.cloned()
        } else {
            None
        },
        callbacks: nested.callbacks,
    }
}

struct ScopedCalls {
    calls: Vec<super::Observation>,
    callbacks: Vec<Observation>,
}

fn callback_calls<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    observe_callbacks: bool,
) -> ScopedCalls {
    let ExprKind::Closure(closure) = expression.kind else {
        unreachable!("validated callback expression")
    };
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
        observe_callbacks,
        callback_owner: Some(reference(expression.hir_id)),
    };
    scan.visit_expr(context.tcx.hir_body(closure.body).value);

    ScopedCalls {
        calls: scan.calls,
        callbacks: scan.callbacks,
    }
}
