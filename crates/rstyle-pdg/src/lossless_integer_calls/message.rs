//! A nonescaping static literal operand of one proved-total expect operation.
use super::*;

pub(crate) const CONSTRUCT: &str = "data.lossless-expect-message";

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if node.kind != "expr_lit"
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
            != Some("{\"kind\":\"shared\",\"value\":{\"kind\":\"str\"}}")
        || ![
            CAPABILITY,
            U32_USIZE_CAPABILITY,
            crate::tryfrom_calls::input::CAPABILITY,
        ]
        .into_iter()
        .any(|capability| crate::extraction_profile::facts_support(facts, capability))
    {
        return Ok(false);
    }

    let mut parents = Vec::new();
    collect(&facts.body, node, &mut parents);
    let [parent] = parents.as_slice() else {
        return Ok(false);
    };
    if !kind(parent, facts).is_some_and(|kind| !kind.is_conversion()) {
        return Ok(false);
    }
    resolve(parent, facts)?.ok_or("Missing total expect for literal diagnostic")?;

    let syntax = crate::source_evidence::original_expression(node, facts)?;
    if !matches!(syntax, syn::Expr::Lit(ref literal) if matches!(literal.lit, syn::Lit::Str(_)))
        || node.expansion.is_some()
        || node.is_unsafe_block
        || !node.children.is_empty()
        || node.lit.as_deref()
            != facts
                .source
                .bytes
                .get(node.span.start as usize..node.span.end as usize)
    {
        return Err("Total expect diagnostic loses its original static literal".into());
    }
    Ok(true)
}

fn collect<'a>(root: &'a HirNode, node: &HirNode, parents: &mut Vec<&'a HirNode>) {
    if root
        .children
        .get(1)
        .is_some_and(|child| child.hir == node.hir && child.span == node.span)
    {
        parents.push(root);
    }
    for child in &root.children {
        collect(child, node, parents);
    }
}
