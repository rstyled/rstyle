//! Exact original body extent; no prefix-only certification or receiver permission.
use super::*;
use syn::spanned::Spanned;

pub(super) fn verify(input: &dyn EvidenceFacts) -> Result<&HirNode, String> {
    let source: syn::ItemFn =
        syn::parse_str(&input.source.bytes).map_err(|error| error.to_string())?;
    if input.signature.has_self
        || input.signature.receiver.is_some()
        || source.sig.receiver().is_some()
        || input.signature.is_async
        || input.signature.is_unsafe
        || input.signature.is_variadic
        || source.sig.asyncness.is_some()
        || source.sig.unsafety.is_some()
        || source.sig.variadic.is_some()
        || source.sig.abi.is_some()
        || !source.attrs.is_empty()
    {
        return Err(
            "Digit product activation requires an original ordinary receiver-free function".into(),
        );
    }
    let [syn::Stmt::Expr(syn::Expr::Match(original), None)] = source.block.stmts.as_slice() else {
        return Err("Digit product activation requires its complete single-match body".into());
    };
    let root = &input.body;
    let [matched] = root.children.as_slice() else {
        return Err("Digit product body cannot omit or append statements".into());
    };
    if root.kind != "expr_block"
        || root.expansion.is_some()
        || root.dispatch.is_some()
        || root.is_unsafe_block
        || root.span
            != crate::source_evidence::original_span(&input.source.bytes, source.block.span())
        || matched.span
            != crate::source_evidence::original_span(&input.source.bytes, original.span())
        || crate::source_evidence::byte_range(&input.source.bytes, root.span)? != root.snippet
        || crate::source_evidence::byte_range(&input.source.bytes, matched.span)? != matched.snippet
    {
        return Err("Digit product body extent differs from its complete original source".into());
    }

    Ok(matched)
}
