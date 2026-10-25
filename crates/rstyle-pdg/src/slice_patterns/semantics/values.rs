//! Borrowed pattern bindings and the distinct original success-arm content loads.
use super::*;

pub(super) fn digits<'a>(
    prefix: &CheckedPrefix<'a>,
    body: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Digit<'a>>, String> {
    prefix
        .digits()
        .iter()
        .map(|digit| {
            let pointer = unique_use(body, digit.binding(), facts)?;
            let reads = crate::field_locations::operations(body, &["expr_unary"]);
            let mut reads = reads
                .into_iter()
                .filter(|read| read.children.len() == 1 && read.children[0] == *pointer);
            let read = reads
                .next()
                .ok_or("Digit pointer lacks its distinct original dereference")?;
            if reads.next().is_some() {
                return Err("Digit pointer is loaded more than once in the selected body".into());
            }

            crate::data_dereferences::verify(read, facts)?;
            if crate::data_aggregates::token(read)? != "u8" {
                return Err("Digit dereference changed its original byte type".into());
            }
            Ok(Digit {
                binding: digit.binding(),
                range: digit.range(),
                pointer,
                read,
            })
        })
        .collect()
}

pub(super) fn remainder<'a>(
    prefix: &CheckedPrefix<'a>,
    body: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    unique_use(body, prefix.remainder().binding(), facts)
}

fn unique_use<'a>(
    body: &'a HirNode,
    binding: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    let mut uses = crate::field_locations::operations(body, &["expr_path"])
        .into_iter()
        .filter(|node| {
            node.res
                .as_ref()
                .and_then(|resolution| resolution.target.as_ref())
                == Some(&binding.hir)
        });
    let pointer = uses.next().ok_or("Missing original pattern binding use")?;
    if uses.next().is_some()
        || crate::source_evidence::resolved_local_leaf(pointer) != Some(&binding.hir)
        || pointer.hir.owner != binding.hir.owner
        || pointer.hir.krate != 0
        || crate::data_aggregates::token(pointer)? != crate::data_aggregates::token(binding)?
    {
        return Err("Pattern reference requires one exact original success-arm use".into());
    }
    let original = facts
        .bindings
        .iter()
        .find(|entry| entry.hir == binding.hir)
        .ok_or("Missing original pattern binding name")?;

    let syntax = crate::source_evidence::original_expression(pointer, facts)?;
    if !matches!(syntax, syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none()
        && crate::source_evidence::plain_relative_path(&path.path) == Some(vec![original.name.clone()]))
    {
        return Err("Pattern reference no longer names its original binding".into());
    }
    Ok(pointer)
}
