//! Exact source grammar supplies the rank, exit condition and update order.
use crate::{ByteSpan, unit_store::EvidenceFacts};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::{ext::IdentExt, spanned::Spanned};

pub(super) struct Shape {
    pub loop_span: ByteSpan,
    pub divide: ByteSpan,
    pub remainder: ByteSpan,
    pub break_span: ByteSpan,
    pub value_update: ByteSpan,
    pub count_update: ByteSpan,
}

struct Binding<'a> {
    syntax: &'a syn::PatIdent,
    parameter: bool,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Shape, String> {
    let source: &str = &facts.source.bytes;
    let function: syn::ItemFn = syn::parse_str(source).map_err(|error| error.to_string())?;
    let (shape, bindings) = parse(source, &function)?;
    if facts.bindings.len() != bindings.len()
        || facts.signature.params.len() != 1
        || facts.signature.return_ty.normalized.as_deref() != Some("u32")
    {
        return Err("Bounded scalar helper has an incomplete binding/signature inventory".into());
    }

    let mut resolved = Vec::new();
    for expected in bindings {
        let captured = binding(facts, source, &expected)?;
        if resolved
            .iter()
            .any(|(_, previous): &(&syn::PatIdent, &crate::BindingFact)| {
                previous.symbol == captured.symbol || previous.hir == captured.hir
            })
        {
            return Err("Bounded scalar helper merges distinct source binding identities".into());
        }
        resolved.push((expected.syntax, captured));
    }
    references(&facts.body, facts, &resolved)?;
    Ok(shape)
}

fn binding<'a>(
    facts: &'a dyn EvidenceFacts,
    source: &str,
    expected: &Binding<'_>,
) -> Result<&'a crate::BindingFact, String> {
    let span = crate::source_evidence::original_span(source, expected.syntax.ident.span());
    let candidates: Vec<_> = facts
        .bindings
        .iter()
        .filter(|binding| binding.decl_span == span)
        .collect();
    let [binding] = candidates.as_slice() else {
        return Err("Bounded scalar helper lacks one exact source binding".into());
    };

    if binding.is_param != expected.parameter
        || binding.is_mut != expected.syntax.mutability.is_some()
        || binding.ty.normalized.as_deref() != Some("u32")
        || expected.parameter && facts.signature.params[0].symbol != binding.symbol
    {
        return Err("Bounded scalar helper source binding disagrees with captured identity".into());
    }
    Ok(binding)
}

fn references(
    node: &crate::HirNode,
    facts: &dyn EvidenceFacts,
    bindings: &[(&syn::PatIdent, &crate::BindingFact)],
) -> Result<(), String> {
    if node.kind == "expr_path" {
        let bytes = facts
            .source
            .bytes
            .get(node.span.start as usize..node.span.end as usize)
            .ok_or("Bounded scalar helper reference lacks original source")?;
        let name: syn::Ident = syn::parse_str(bytes)
            .map_err(|_| "Bounded scalar helper reference is not a plain original local")?;
        let (_, binding) = bindings
            .iter()
            .find(|(syntax, _)| syntax.ident.unraw() == name.unraw())
            .ok_or("Bounded scalar helper reference has no lexical source binding")?;
        if node.res.as_ref().is_none_or(|resolution| {
            resolution.kind != "local" || resolution.target.as_ref() != Some(&binding.hir)
        }) || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u32")
        {
            return Err(
                "Bounded scalar helper reference disagrees with its lexical source binding".into(),
            );
        }
    }
    for child in &node.children {
        references(child, facts, bindings)?;
    }
    Ok(())
}

fn parse<'a>(source: &str, function: &'a syn::ItemFn) -> Result<(Shape, Vec<Binding<'a>>), String> {
    signature(function)?;
    let Some(syn::FnArg::Typed(parameter)) = function.sig.inputs.first() else {
        return Err("Bounded scalar helper requires exactly one scalar argument".into());
    };
    let value = pattern(&parameter.pat)?;
    let [count, syn::Stmt::Expr(syn::Expr::Loop(looped), None), _] =
        function.block.stmts.as_slice()
    else {
        return Err("Bounded scalar helper requires initialization, loop and final count".into());
    };
    let count = local(count)?;
    let [
        _,
        quotient,
        remainder,
        syn::Stmt::Expr(syn::Expr::If(branch), None),
        update,
        increment,
    ] = looped.body.stmts.as_slice()
    else {
        return Err("Bounded scalar helper loop has unproved statements or transfers".into());
    };

    let quotient = local(quotient)?;
    let remainder = local(remainder)?;
    exact_body(source, [value, count, quotient, remainder])?;
    let bindings = [value, count, quotient, remainder]
        .into_iter()
        .enumerate()
        .map(|(index, syntax)| Binding {
            syntax,
            parameter: index == 0,
        })
        .collect();
    let shape = Shape {
        loop_span: span(source, looped),
        divide: initializer_span(source, &looped.body.stmts[1])?,
        remainder: initializer_span(source, &looped.body.stmts[2])?,
        break_span: expression_span(source, &branch.then_branch.stmts[0])?,
        value_update: expression_span(source, update)?,
        count_update: expression_span(source, increment)?,
    };
    Ok((shape, bindings))
}

fn signature(function: &syn::ItemFn) -> Result<(), String> {
    let signature = &function.sig;
    if !function.attrs.is_empty()
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.inputs.len() != 1
        || !matches!(signature.inputs.first(), Some(syn::FnArg::Typed(parameter)) if crate::source_evidence::plain_type_path(&parameter.ty, "u32"))
        || !matches!(&signature.output, syn::ReturnType::Type(_, ty) if crate::source_evidence::plain_type_path(ty, "u32"))
    {
        return Err("Bounded scalar helper requires a plain concrete Rust function".into());
    }
    Ok(())
}

fn pattern(pattern: &syn::Pat) -> Result<&syn::PatIdent, String> {
    match pattern {
        syn::Pat::Ident(binding) if binding.by_ref.is_none() && binding.subpat.is_none() => {
            Ok(binding)
        }
        _ => Err("Bounded scalar helper requires simple independent bindings".into()),
    }
}

fn local(statement: &syn::Stmt) -> Result<&syn::PatIdent, String> {
    match statement {
        syn::Stmt::Local(local) if local.init.is_some() => pattern(&local.pat),
        _ => Err("Bounded scalar helper requires initialized scalar locals".into()),
    }
}

fn exact_body(source: &str, names: [&syn::PatIdent; 4]) -> Result<(), String> {
    let [value, count, quotient, remainder] = names.map(|binding| binding.ident.to_string());
    let distinct: std::collections::BTreeSet<_> = names
        .iter()
        .map(|binding| binding.ident.unraw().to_string())
        .collect();
    if distinct.len() != 4
        || names[0].mutability.is_none()
        || names[1].mutability.is_none()
        || names[2].mutability.is_some()
        || names[3].mutability.is_some()
    {
        return Err("Bounded scalar helper bindings shadow or mutate the wrong state".into());
    }

    let expected = format!(
        "{{ let mut {count} = 0u32; loop {{ debug_assert!({value} != 0); let {quotient} = {value} / 5; let {remainder} = {value} % 5; if {remainder} != 0 {{ break; }} {value} = {quotient}; {count} += 1; }} {count} }}"
    );
    if body_tokens(source)? != body_tokens(&expected)? {
        return Err(
            "Bounded scalar helper lacks the complete checked factor-counting source grammar"
                .into(),
        );
    }
    Ok(())
}

fn body_tokens(source: &str) -> Result<String, String> {
    let stream: TokenStream = source
        .parse()
        .map_err(|error: proc_macro2::LexError| error.to_string())?;
    match stream.into_iter().last() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
            Ok(group.stream().to_string())
        }
        _ => Err("Missing original bounded scalar helper body".into()),
    }
}

fn span(source: &str, syntax: &impl Spanned) -> ByteSpan {
    crate::source_evidence::original_span(source, syntax.span())
}

fn initializer_span(source: &str, statement: &syn::Stmt) -> Result<ByteSpan, String> {
    let syn::Stmt::Local(local) = statement else {
        return Err("Missing loop local".into());
    };
    Ok(span(
        source,
        &local.init.as_ref().ok_or("Missing loop initializer")?.expr,
    ))
}

fn expression_span(source: &str, statement: &syn::Stmt) -> Result<ByteSpan, String> {
    let syn::Stmt::Expr(expression, _) = statement else {
        return Err("Missing loop update".into());
    };
    Ok(span(source, expression))
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "fn factor(mut value:u32)->u32 {let mut count=0u32; loop {debug_assert!(value != 0); let q=value/5; let r=value%5; if r!=0 {break;} value=q; count+=1;} count}";

    #[test]
    fn source_rank_requires_complete_ordered_body() {
        let function = syn::parse_str(SOURCE).unwrap();
        assert!(parse(SOURCE, &function).is_ok());
        for (old, new) in [
            ("value/5", "value/7"),
            ("value%5", "q%5"),
            ("r!=0", "r==0"),
            ("count+=1", "count+=2"),
            ("value=q; count+=1", "count+=1; value=q"),
            ("debug_assert!(value != 0);", ""),
            ("break;", "continue;"),
            (" count}", " value}"),
            ("value=q", "value=value"),
            ("count=0u32", "count=1u32"),
        ] {
            let changed = SOURCE.replace(old, new);
            let function = syn::parse_str(&changed).unwrap();
            assert!(parse(&changed, &function).is_err(), "{old} → {new}");
        }
    }

    #[test]
    fn alpha_names_preserve_rank_but_shadowing_does_not() {
        let changed = SOURCE.replace("value", "input").replace("count", "factors");
        assert!(parse(&changed, &syn::parse_str(&changed).unwrap()).is_ok());
        let changed = SOURCE
            .replace("let r=value%5", "let q=value%5")
            .replace("r!=0", "q!=0");
        assert!(parse(&changed, &syn::parse_str(&changed).unwrap()).is_err());
        let changed = SOURCE
            .replace("let r=value%5", "let r#q=value%5")
            .replace("r!=0", "r#q!=0");
        assert!(parse(&changed, &syn::parse_str(&changed).unwrap()).is_err());
    }
}
