//! Same-module literal usize constants in selected nominal field lengths.
//! The checked module already owns the source and cfg/import closure. No second
//! constant inventory, compiler observation or evaluated-expression cache is used.

use super::*;
use concrete_types::source_proof::literal_length;

pub(super) fn verify(scope: &Scoped<'_>, source: &syn::Expr, expected: u64) -> Result<(), String> {
    if let syn::Expr::Lit(_) = source {
        return literal_length(source, expected);
    }
    if !scope.resolution.array_views {
        return Err("Named array lengths require their registered capability".into());
    }
    let syn::Expr::Path(path) = source else {
        return Err("Array length requires a literal or same-module constant".into());
    };
    let name = path
        .path
        .get_ident()
        .ok_or("Array length requires a bare constant name")?;
    if path.qself.is_some()
        || scope
            .resolution
            .environment
            .index
            .imports
            .contains_key(&(scope.namespace.clone(), name.to_string()))
    {
        return Err("Array length cannot use an imported or qualified constant".into());
    }

    let constant = declaration(scope, name)?;
    if let syn::Expr::Lit(expression) = constant.expr.as_ref()
        && let syn::Lit::Int(value) = &expression.lit
        && matches!(value.suffix(), "" | "usize")
    {
        return literal_length(&constant.expr, expected);
    }
    Err("Array length constant requires an exact usize literal initializer".into())
}

fn declaration<'a>(scope: &'a Scoped<'_>, name: &syn::Ident) -> Result<&'a syn::ItemConst, String> {
    let mut candidates = scope
        .resolution
        .environment
        .modules
        .iter()
        .filter(|module| &module.namespace == scope.namespace)
        .flat_map(|module| &module.items)
        .filter_map(|item| match item {
            syn::Item::Const(item) if item.ident == *name => Some(item),
            _ => None,
        });
    let constant = candidates
        .next()
        .ok_or("Missing same-module array length constant")?;
    if candidates.next().is_some()
        || !constant.generics.params.is_empty()
        || !matches!(constant.ty.as_ref(), syn::Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("usize"))
    {
        return Err("Array length requires one concrete usize constant declaration".into());
    }
    Ok(constant)
}
