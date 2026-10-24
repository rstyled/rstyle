use super::*;

pub(super) fn verify<'a>(node: &'a HirNode, context: &Context<'_>) -> Result<Match<'a>, String> {
    let checked = shape(node, context)?;
    results(context, checked.binding, checked.success, checked.failure)?;
    Ok(checked)
}

pub(super) fn shape<'a>(node: &'a HirNode, context: &Context<'_>) -> Result<Match<'a>, String> {
    if context.endpoints.error.is_none() || node.kind != "expr_match" {
        return Err("Copy conversion requires its original inverse match".into());
    }
    let syntax = crate::source_evidence::original_expression(node, context.facts)?;
    let syn::Expr::Match(syntax) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Copy conversion inverse lost its match syntax".into());
    };
    let [scrutinee, success_arm, failure_arm] = node.children.as_slice() else {
        return Err("Copy conversion requires exactly the two original arms".into());
    };
    if syntax.arms.len() != 2 || node.expansion.is_some() {
        return Err("Copy conversion inverse must retain both original arms".into());
    }

    context.typed(node, context.endpoints.output)?;
    context.typed(scrutinee, context.endpoints.tagged)?;
    context.formal(scrutinee)?;
    context.correspondence(syntax.expr.span(), scrutinee)?;
    let (pattern, success) = arm(context, &syntax.arms[0], success_arm)?;
    let (wildcard, failure) = arm(context, &syntax.arms[1], failure_arm)?;
    let binding = binding(context, &syntax.arms[0].pat, pattern)?;
    let tag = context.variant(pattern, "resolved_local_variant")?;
    context.typed(pattern, context.endpoints.tagged)?;
    wildcard_pattern(context, &syntax.arms[1].pat, wildcard)?;
    result_values(context, binding, success, failure)?;
    Ok(Match {
        scrutinee,
        pattern,
        binding,
        success,
        wildcard,
        failure,
        tag,
    })
}

fn arm<'a>(
    context: &Context<'_>,
    syntax: &syn::Arm,
    node: &'a HirNode,
) -> Result<(&'a HirNode, &'a HirNode), String> {
    let [pattern, body] = node.children.as_slice() else {
        return Err("Copy conversion arm requires its exact pattern and body".into());
    };
    if node.kind != "match_arm"
        || node.expansion.is_some()
        || !syntax.attrs.is_empty()
        || syntax.guard.is_some()
    {
        return Err("Copy conversion arm loses original unguarded syntax".into());
    }

    context.correspondence(syntax.pat.span(), pattern)?;
    context.correspondence(syntax.body.span(), body)?;
    Ok((pattern, body))
}

fn binding<'a>(
    context: &Context<'_>,
    syntax: &syn::Pat,
    pattern: &'a HirNode,
) -> Result<&'a HirNode, String> {
    let syn::Pat::TupleStruct(syntax) = syntax else {
        return Err("Copy conversion requires the original tuple payload pattern".into());
    };
    let [node] = pattern.children.as_slice() else {
        return Err("Copy conversion requires one whole-product binding".into());
    };
    if syntax.elems.len() != 1 {
        return Err("Copy conversion requires one immutable identifier".into());
    }
    let syn::Pat::Ident(ident) = &syntax.elems[0] else {
        return Err("Copy conversion requires one immutable identifier".into());
    };
    if pattern.kind != "pat_local_variant"
        || syntax.qself.is_some()
        || ident.by_ref.is_some()
        || ident.mutability.is_some()
        || ident.subpat.is_some()
    {
        return Err("Copy conversion payload is not a whole-product immutable binding".into());
    }

    binding_fact(context, node, &ident.ident.to_string())?;
    context.correspondence(syntax.elems[0].span(), node)?;
    Ok(node)
}

fn binding_fact(context: &Context<'_>, node: &HirNode, name: &str) -> Result<(), String> {
    context.typed(node, context.endpoints.product)?;
    let fact = context
        .facts
        .bindings
        .iter()
        .find(|fact| Some(&fact.symbol) == node.binding.as_ref())
        .ok_or("Missing conversion payload binding fact")?;
    if node.kind != "pat_binding"
        || node.binding_mut != Some(false)
        || !node.children.is_empty()
        || fact.is_mut
        || fact.is_param
        || fact.hir != node.hir
        || fact.name != name
        || fact.decl_span != node.span
        || fact.ty.normalized.as_deref() != Some(crate::data_aggregates::token(node)?)
    {
        return Err("Copy conversion payload binding declaration disagrees".into());
    }
    Ok(())
}

fn wildcard_pattern(
    context: &Context<'_>,
    syntax: &syn::Pat,
    node: &HirNode,
) -> Result<(), String> {
    context.typed(node, context.endpoints.tagged)?;
    if !matches!(syntax, syn::Pat::Wild(_))
        || node.kind != "pat_wild"
        || !node.children.is_empty()
        || node.res.is_some()
        || node.binding.is_some()
    {
        return Err("Copy conversion complement must retain the original wildcard".into());
    }
    Ok(())
}

fn result_values(
    context: &Context<'_>,
    binding: &HirNode,
    success: &HirNode,
    failure: &HirNode,
) -> Result<(), String> {
    context.typed(success, context.endpoints.output)?;
    context.typed(failure, context.endpoints.output)?;
    let [ok] = success.children.as_slice() else {
        return Err("Copy conversion Ok must carry one complete payload".into());
    };
    let [err] = failure.children.as_slice() else {
        return Err("Copy conversion Err must carry one exact nominal error".into());
    };
    if success.kind != "expr_call"
        || failure.kind != "expr_call"
        || ok.kind != "expr_path"
        || !ok.children.is_empty()
        || !ok
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "local" && res.target.as_ref() == Some(&binding.hir))
        || err.kind != "expr_struct"
        || !err.children.is_empty()
    {
        return Err(
            "Copy conversion Result operands change the original binding or unit error".into(),
        );
    }

    context.typed(ok, context.endpoints.product)?;
    context.typed(
        err,
        context.endpoints.error.ok_or("Missing conversion error")?,
    )
}

fn results(
    context: &Context<'_>,
    binding: &HirNode,
    success: &HirNode,
    failure: &HirNode,
) -> Result<(), String> {
    context.typed(success, context.endpoints.output)?;
    context.typed(failure, context.endpoints.output)?;
    let ok = crate::concrete_results::verify_constructor(success, context.facts)?;
    let err = crate::concrete_results::verify_constructor(failure, context.facts)?;
    if ok.construct != "data.result-ok"
        || err.construct != "data.result-err"
        || ok.payload.kind != "expr_path"
        || !ok.payload.children.is_empty()
        || !ok
            .payload
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "local" && res.target.as_ref() == Some(&binding.hir))
    {
        return Err("Copy conversion Result constructors lose the original alternatives".into());
    }

    context.typed(ok.payload, context.endpoints.product)?;
    context.typed(
        err.payload,
        context.endpoints.error.ok_or("Missing conversion error")?,
    )?;
    if err.payload.kind != "expr_struct" || !err.payload.children.is_empty() {
        return Err("Copy conversion error must be its exact nominal unit struct".into());
    }
    crate::data_structs::slots(err.payload, context.facts)?;
    Ok(())
}
