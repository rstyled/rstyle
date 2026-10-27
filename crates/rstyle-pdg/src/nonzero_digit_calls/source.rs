//! Complete bounded caller syntax; lexical identities and effects remain independent.
use crate::{HirFnFacts, scalar_product_instances::Argument};
use syn::spanned::Spanned;

pub(crate) fn shape(facts: &HirFnFacts, argument: Argument) -> Result<(), String> {
    let function: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;

    let signature = &function.sig;

    if signature.receiver().is_some()
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.inputs.len() != 1
    {
        return Err(
            "Nonzero composition requires a complete ordinary one-input free function".into(),
        );
    }

    let width = match argument {
        Argument::NonZeroU8 => 2,
        Argument::NonZeroU16 => 3,
        _ => return Err("Nonzero composition source has another substitution".into()),
    };

    let expected = format!(
        "{{ ExactlyNDigits::<{width}>::parse(input).and_then(|parsed| parsed.flat_map(NonZero::new)) }}"
    );

    same_tokens(&facts.source.bytes, function.block.span(), &expected)?;

    let Some(syn::FnArg::Typed(input)) = signature.inputs.first() else {
        return Err("Nonzero composition lacks its original shared input".into());
    };

    if !matches!(input.pat.as_ref(), syn::Pat::Ident(binding) if binding.ident == "input" && binding.by_ref.is_none() && binding.mutability.is_none() && binding.subpat.is_none() && binding.attrs.is_empty())
    {
        return Err("Nonzero composition changes its saved input binding".into());
    }

    same_tokens(&facts.source.bytes, input.ty.span(), "&[u8]")?;

    let syn::ReturnType::Type(_, output) = &signature.output else {
        return Err("Nonzero caller lacks its complete result spelling".into());
    };

    let scalar = if width == 2 { "u8" } else { "u16" };

    same_tokens(
        &facts.source.bytes,
        output.span(),
        &format!("Option<ParsedItem<'_, NonZero<{scalar}>>>"),
    )
}

fn same_tokens(source: &str, span: proc_macro2::Span, expected: &str) -> Result<(), String> {
    let span = crate::source_evidence::original_span(source, span);

    let source = crate::source_evidence::byte_range(source, span)?;

    let tokens = |source: &str| {
        source
            .parse::<proc_macro2::TokenStream>()
            .map(|stream| stream.to_string())
            .map_err(|error| error.to_string())
    };

    if tokens(source)? != tokens(expected)? {
        return Err(
            "Nonzero composition changes its complete original body or interface spelling".into(),
        );
    }

    Ok(())
}

pub(super) fn correspondence(
    facts: &dyn crate::unit_store::EvidenceFacts,
    calls: &super::selection::Calls<'_>,
) -> Result<(), String> {
    let source: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;

    let [syn::Stmt::Expr(syn::Expr::MethodCall(outer), None)] = source.block.stmts.as_slice()
    else {
        return Err("Missing original complete nonzero expression".into());
    };

    let [syn::Expr::Closure(callback)] = outer.args.iter().collect::<Vec<_>>().as_slice() else {
        return Err("Missing original exact callback".into());
    };

    let syn::Expr::MethodCall(nested) = callback.body.as_ref() else {
        return Err("Missing original complete nested member call".into());
    };

    for (syntax, node) in [
        (outer.span(), calls.outer),
        (outer.receiver.span(), calls.parser),
        (callback.span(), calls.callback),
    ] {
        if crate::source_evidence::original_span(&facts.source.bytes, syntax) != node.span {
            return Err("Nonzero callback source/HIR call spans disagree".into());
        }
    }

    if calls.nested["span"]
        != serde_json::json!(crate::source_evidence::original_span(
            &facts.source.bytes,
            nested.span()
        ))
    {
        return Err(
            "Separately owned callback call differs from its complete original source".into(),
        );
    }

    let actual: crate::source_evidence::Correspondence = serde_json::from_value(
        facts
            .build
            .frontend_options
            .get(crate::source_evidence::KEY)
            .ok_or("Missing source correspondence")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;

    if actual != crate::source_evidence::capture(facts)? {
        return Err("Nonzero caller changes its exact original source inventory".into());
    }
    findings(facts, calls, nested.span())
}

fn findings(
    facts: &dyn crate::unit_store::EvidenceFacts,
    calls: &super::selection::Calls<'_>,
    nested: proc_macro2::Span,
) -> Result<(), String> {
    let spans = [
        calls.outer.span,
        crate::source_evidence::original_span(&facts.source.bytes, nested),
    ];

    if facts.source_diagnostics.len() != spans.len() {
        return Err("Nonzero caller requires its exact preserved source finding roster".into());
    }

    for (finding, span) in facts.source_diagnostics.iter().zip(spans) {
        if finding.span != span
            || finding.capability != "rust.call-summaries"
            || finding.construct != "expr_method_call"
            || finding.severity != "error"
            || finding.snippet != crate::source_evidence::byte_range(&facts.source.bytes, span)?
            || finding.reason
                != "Original owned source contains deferred rust.call-summaries; cfg removal cannot certify a supported remainder."
        {
            return Err(
                "Nonzero composition cannot cover unrelated or missing source findings".into(),
            );
        }
    }

    Ok(())
}

pub(super) fn binding(
    facts: &dyn crate::unit_store::EvidenceFacts,
    actual: &crate::HirNode,
    formal: &crate::frontend::ParamFact,
) -> Result<(), String> {
    if crate::source_evidence::resolved_local_leaf(actual) != Some(&formal.pat.hir)
        || formal.position != 0
        || formal.pat.kind != "pat_binding"
        || !formal.pat.children.is_empty()
        || formal.pat.binding.as_deref() != Some(formal.symbol.as_str())
        || formal.pat.binding_mut != Some(false)
        || actual
            .ty
            .as_ref()
            .is_none_or(|ty| ty.normalized != formal.ty.normalized)
    {
        return Err("Nonzero caller changes its original once-only shared input binding".into());
    }

    let source: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;

    let syn::FnArg::Typed(parameter) = &source.sig.inputs[0] else {
        return Err("Missing original formal".into());
    };

    if formal.pat.span
        != crate::source_evidence::original_span(&facts.source.bytes, parameter.pat.span())
    {
        return Err("Nonzero caller formal/source ownership disagrees".into());
    }

    Ok(())
}
