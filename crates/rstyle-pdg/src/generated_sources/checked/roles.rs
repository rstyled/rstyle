//! Complete parameter/body correspondence for a bounded direct-call expression tree.
use super::*;
use crate::{HirNode, conditional_source::Configuration};

pub(super) fn verify(
    input: &Input<'_>,
    observation: &Observation,
    function: &syn::ImplItemFn,
    expansion: &source::CheckedExpansion,
) -> Result<Vec<CheckedRole>, String> {
    function_source(input, observation, function, expansion)?;
    let expected = syntax(function)?;
    let facts = fact_roles(input.facts)?;
    if facts.len() != expected.len() || observation.nodes.len() != expected.len() {
        return Err("Generated source roles do not cover the complete facts tree".into());
    }

    let mut result = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for ((expected, (path, node)), observed) in expected.iter().zip(facts).zip(&observation.nodes) {
        if expected.path != path
            || observed.path != path
            || expected.kind != node.kind
            || observed.kind != node.kind
            || observed.hir != node.hir
            || node.hir.krate != 0
            || node.hir.owner != input.facts.body.hir.owner
            || !seen.insert((node.hir.owner, node.hir.local))
        {
            return Err("Generated exact source/facts/HIR role inventories disagree".into());
        }
        let source = expansion.source_span(expected.span);
        origin(&observed.origin, &source, &observation.defining_expansion)?;
        result.push(CheckedRole {
            path,
            hir: node.hir.clone(),
            kind: node.kind.clone(),
            source,
        });
    }
    Ok(result)
}

fn function_source(
    input: &Input<'_>,
    observation: &Observation,
    function: &syn::ImplItemFn,
    expansion: &source::CheckedExpansion,
) -> Result<(), String> {
    signature(input.facts, function)?;
    roster::attributes(
        &function.attrs,
        &Configuration::new(&input.facts.build.cfgs),
    )?;

    origin(
        &observation.signature,
        &expansion.source_span(visible_span(&function.vis, function.sig.span())?),
        &observation.defining_expansion,
    )?;
    origin(
        &observation.body,
        &expansion.source_span(function.block.span()),
        &observation.defining_expansion,
    )?;
    Ok(())
}

fn signature(facts: &HirFnFacts, function: &syn::ImplItemFn) -> Result<(), String> {
    let syntax = &function.sig;
    let actual = &facts.signature;
    if syntax.asyncness.is_some()
        || syntax.unsafety.is_some()
        || syntax.constness.is_some()
        || syntax.abi.is_some()
        || syntax.variadic.is_some()
        || !syntax.generics.params.is_empty()
        || syntax.generics.where_clause.is_some()
        || actual.is_async
        || actual.is_unsafe
        || actual.is_const
        || actual.is_variadic
        || actual.has_self
        || actual.receiver.is_some()
        || !actual.generics.is_empty()
        || actual.abi != crate::known_str("Rust")
        || syntax.inputs.len() != actual.params.len()
    {
        return Err("Generated function has unsupported or disagreeing signature ownership".into());
    }
    crate::source_evidence::closed_body(&function.block)
}

struct SyntaxRole {
    path: Vec<u32>,
    kind: &'static str,
    span: proc_macro2::Span,
}

fn syntax(function: &syn::ImplItemFn) -> Result<Vec<SyntaxRole>, String> {
    let mut roles = Vec::new();
    for (index, parameter) in function.sig.inputs.iter().enumerate() {
        let syn::FnArg::Typed(parameter) = parameter else {
            return Err("Generated member has a receiver".into());
        };
        let syn::Pat::Ident(pattern) = parameter.pat.as_ref() else {
            return Err("Generated parameter is not a plain binding".into());
        };
        if pattern.by_ref.is_some()
            || pattern.mutability.is_some()
            || pattern.subpat.is_some()
            || !pattern.attrs.is_empty()
        {
            return Err("Generated parameter has unsupported binding syntax".into());
        }
        roles.push(SyntaxRole {
            path: vec![
                0,
                index
                    .try_into()
                    .map_err(|_| "Parameter ordinal exceeds u32")?,
            ],
            kind: "pat_binding",
            span: pattern.ident.span(),
        });
    }

    roles.push(SyntaxRole {
        path: vec![1],
        kind: "expr_block",
        span: function.block.span(),
    });
    let [syn::Stmt::Expr(expression, None)] = function.block.stmts.as_slice() else {
        return Err("Generated member requires one complete tail expression".into());
    };
    expression_roles(expression, vec![1, 0], &mut roles)?;
    Ok(roles)
}

fn expression_roles(
    expression: &syn::Expr,
    path: Vec<u32>,
    roles: &mut Vec<SyntaxRole>,
) -> Result<(), String> {
    if roles.len() >= 32768
        || path.len() > 256
        || !crate::conditional_source::expression_attributes(expression).is_empty()
    {
        return Err("Generated expression exceeds the bounded source grammar".into());
    }
    let (kind, children): (_, Vec<&syn::Expr>) = match expression {
        syn::Expr::Path(value)
            if value.qself.is_none()
                && crate::source_evidence::plain_relative_path(&value.path).is_some() =>
        {
            ("expr_path", vec![])
        }
        syn::Expr::Call(call)
            if matches!(call.func.as_ref(), syn::Expr::Path(path)
            if path.qself.is_none() && crate::source_evidence::plain_relative_path(&path.path).is_some()) =>
        {
            ("expr_call", call.args.iter().collect())
        }
        syn::Expr::MethodCall(call) if call.turbofish.is_none() => (
            "expr_method_call",
            std::iter::once(call.receiver.as_ref())
                .chain(call.args.iter())
                .collect(),
        ),
        _ => return Err("Generated member contains unsupported source operations".into()),
    };
    roles.push(SyntaxRole {
        path: path.clone(),
        kind,
        span: expression.span(),
    });

    for (index, child) in children.into_iter().enumerate() {
        let mut child_path = path.clone();
        child_path.push(index.try_into().map_err(|_| "Child ordinal exceeds u32")?);
        expression_roles(child, child_path, roles)?;
    }
    Ok(())
}

fn fact_roles(facts: &HirFnFacts) -> Result<Vec<(Vec<u32>, &HirNode)>, String> {
    let mut roles = Vec::new();
    for (index, parameter) in facts.signature.params.iter().enumerate() {
        collect(
            &parameter.pat,
            vec![
                0,
                index
                    .try_into()
                    .map_err(|_| "Parameter ordinal exceeds u32")?,
            ],
            &mut roles,
        )?;
    }
    collect(&facts.body, vec![1], &mut roles)?;
    Ok(roles)
}

fn collect<'a>(
    node: &'a HirNode,
    path: Vec<u32>,
    roles: &mut Vec<(Vec<u32>, &'a HirNode)>,
) -> Result<(), String> {
    if roles.len() >= 32768 || path.len() > 256 {
        return Err("Generated facts tree exceeds source budget".into());
    }
    roles.push((path.clone(), node));
    for (index, child) in node.children.iter().enumerate() {
        let mut child_path = path.clone();
        child_path.push(index.try_into().map_err(|_| "Child ordinal exceeds u32")?);
        collect(child, child_path, roles)?;
    }
    Ok(())
}
