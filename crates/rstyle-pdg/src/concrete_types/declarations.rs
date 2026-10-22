//! Independent checks for source-spelled type structure. Named paths and const
//! expressions still require resolved provenance before any data admission.

use super::{TypeTree, scalar};

pub(super) fn check(source: &syn::Type, resolved: &TypeTree) -> Result<(), String> {
    match (source, resolved) {
        (syn::Type::Paren(source), _) => check(&source.elem, resolved),
        (syn::Type::Group(source), _) => check(&source.elem, resolved),
        (syn::Type::Path(source), _) => check_path(source, resolved),
        (syn::Type::Tuple(source), TypeTree::Scalar(token))
            if source.elems.is_empty() && token == "unit" =>
        {
            Ok(())
        }
        (syn::Type::Tuple(source), TypeTree::Tuple(elements)) => {
            check_tuple(&source.elems, elements)
        }
        (syn::Type::Reference(source), TypeTree::Shared(referent))
            if source.mutability.is_none() =>
        {
            check(&source.elem, referent)
        }
        (syn::Type::Slice(source), TypeTree::Slice(element)) => check(&source.elem, element),
        (syn::Type::Array(source), TypeTree::Array { element, length }) => {
            check_array(source, element, *length)
        }
        _ => Err("Declared concrete field shape contradicts resolved type evidence".into()),
    }
}

fn check_path(source: &syn::TypePath, resolved: &TypeTree) -> Result<(), String> {
    if matches!(resolved, TypeTree::RangeUsize { .. }) {
        // Only observational shape here; exact lexical identity is required by
        // source_proof before any spelling can authorize data extraction.
        return Ok(());
    }
    if let TypeTree::Option(payload) = resolved
        && matches!(
            payload.as_ref(),
            TypeTree::Scalar(_) | TypeTree::Tuple(_) | TypeTree::Struct { .. }
        )
    {
        return check(super::option_spelling::payload(source, payload)?, payload);
    }
    if let TypeTree::Result { ok, err } = resolved {
        let [source_ok, source_err] = super::result_arguments(source)?;
        check(source_ok, ok)?;
        return check(source_err, err);
    }
    // A named path may be a resolved alias. Spelling alone proves neither
    // nominal identity nor the alias referent; this observation cannot admit it.
    if source.qself.is_some()
        || source.path.leading_colon.is_some()
        || source.path.segments.len() != 1
    {
        return Ok(());
    }
    super::lifetimes::verify_path(source, resolved)?;
    let segment = &source.path.segments[0];
    let token = segment.ident.to_string();
    if scalar(&token) && resolved != &TypeTree::Scalar(token.clone()) {
        return Err("Declared primitive field type contradicts resolved scalar evidence".into());
    }
    if token == "str" && resolved != &TypeTree::Str {
        return Err("Declared str field type contradicts resolved referent evidence".into());
    }

    Ok(())
}

fn check_tuple(
    source: &syn::punctuated::Punctuated<syn::Type, syn::Token![,]>,
    resolved: &[TypeTree],
) -> Result<(), String> {
    if source.len() != resolved.len() {
        return Err("Declared tuple field arity contradicts resolved type evidence".into());
    }
    for (source, resolved) in source.iter().zip(resolved) {
        check(source, resolved)?;
    }

    Ok(())
}

fn check_array(source: &syn::TypeArray, element: &TypeTree, length: u64) -> Result<(), String> {
    check(&source.elem, element)?;
    // No evaluation of const expressions: literal checks are independent;
    // other lengths remain observational compiler facts, not source proofs.
    if let syn::Expr::Lit(literal) = &source.len
        && let syn::Lit::Int(integer) = &literal.lit
        && integer.base10_parse::<u64>().ok() != Some(length)
    {
        return Err("Declared array field length contradicts resolved type evidence".into());
    }

    Ok(())
}
