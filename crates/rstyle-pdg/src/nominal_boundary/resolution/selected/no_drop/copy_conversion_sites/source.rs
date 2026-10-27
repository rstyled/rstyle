//! Exact source transforms; actual constructor/tag resolution is a value gate.
use super::*;

pub(super) fn function(function: &syn::ItemFn, endpoints: &Endpoints) -> Result<(), String> {
    let name = formal(&function.sig)?;
    let [syn::Stmt::Expr(body, None)] = function.block.stmts.as_slice() else {
        return Err("Copy conversion requires its complete single-expression body".into());
    };
    let TypeTree::MixedEnum { variants, .. } = endpoints.tagged() else {
        return Err("Missing tagged conversion type".into());
    };

    let variant = variants.get(1).ok_or("Missing complete payload variant")?;
    match endpoints.direction {
        Direction::Wrap => wrap(body, name, &variant.name),
        Direction::Unwrap => unwrap(body, name, &variant.name),
    }
}

fn formal(signature: &syn::Signature) -> Result<&syn::Ident, String> {
    if !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.constness.is_some()
    {
        return Err("Copy conversion requires its safe nongeneric Rust member signature".into());
    }

    let Some(syn::FnArg::Typed(argument)) = signature.inputs.first() else {
        return Err("Copy conversion requires one original typed formal".into());
    };
    let syn::Pat::Ident(binding) = argument.pat.as_ref() else {
        return Err("Copy conversion formal must be a plain original binding".into());
    };
    if signature.inputs.len() != 1
        || !argument.attrs.is_empty()
        || !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
    {
        return Err("Copy conversion formal cannot destructure or reborrow".into());
    }
    Ok(&binding.ident)
}

fn wrap(body: &syn::Expr, parameter: &syn::Ident, variant: &str) -> Result<(), String> {
    let argument = call(body, &["Self", variant])?;
    if !crate::source_evidence::plain_value_path(argument, &parameter.to_string()) {
        return Err("Wrapping must preserve exactly its supplied value".into());
    }
    Ok(())
}

fn unwrap(body: &syn::Expr, parameter: &syn::Ident, variant: &str) -> Result<(), String> {
    let syn::Expr::Match(value) = body else {
        return Err("Inverse conversion requires the exact saved tag match".into());
    };
    if value.arms.len() != 2 {
        return Err("Inverse conversion requires its success and complement arms".into());
    }

    let some = &value.arms[0];
    let other = &value.arms[1];
    if !value.attrs.is_empty()
        || !crate::source_evidence::plain_value_path(&value.expr, &parameter.to_string())
        || !some.attrs.is_empty()
        || !other.attrs.is_empty()
        || some.guard.is_some()
        || other.guard.is_some()
        || !matches!(&other.pat, syn::Pat::Wild(pattern) if pattern.attrs.is_empty())
    {
        return Err("Inverse conversion changes scrutinee, guards or wildcard complement".into());
    }

    let binding = payload(&some.pat, variant)?;
    if !crate::source_evidence::plain_value_path(call(&some.body, &["Ok"])?, &binding.to_string()) {
        return Err("Inverse success must return its exact bound payload".into());
    }

    let error = call(&other.body, &["Err"])?;
    if !matches!(error, syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none()
        && crate::source_evidence::plain_relative_path(&path.path)
            .is_some_and(|names| names == ["error", "DifferentVariant"]))
    {
        return Err("Inverse failure must construct its original nominal unit error".into());
    }
    Ok(())
}

fn payload<'a>(pattern: &'a syn::Pat, variant: &str) -> Result<&'a syn::Ident, String> {
    let syn::Pat::TupleStruct(pattern) = pattern else {
        return Err("Inverse conversion requires its exact payload variant pattern".into());
    };
    let Some(syn::Pat::Ident(binding)) = pattern.elems.first() else {
        return Err("Inverse conversion requires its arm-local payload binding".into());
    };
    if !pattern.attrs.is_empty()
        || pattern.qself.is_some()
        || pattern.elems.len() != 1
        || !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
        || crate::source_evidence::plain_relative_path(&pattern.path)
            .is_none_or(|names| names != ["TryFromParsed", variant])
    {
        return Err(
            "Inverse conversion payload path/binding differs from original contract".into(),
        );
    }
    Ok(&binding.ident)
}

fn call<'a>(value: &'a syn::Expr, expected: &[&str]) -> Result<&'a syn::Expr, String> {
    let syn::Expr::Call(value) = value else {
        return Err("Conversion requires original direct constructor syntax".into());
    };
    let syn::Expr::Path(path) = value.func.as_ref() else {
        return Err("Conversion constructor must be an original path".into());
    };
    if !value.attrs.is_empty()
        || !path.attrs.is_empty()
        || path.qself.is_some()
        || value.args.len() != 1
        || crate::source_evidence::plain_relative_path(&path.path)
            .is_none_or(|names| names != expected)
    {
        return Err("Conversion constructor path, attributes or arity differs".into());
    }
    value
        .args
        .first()
        .ok_or_else(|| "Missing conversion constructor operand".into())
}
