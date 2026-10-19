//! Resolve immutable shared-reference aliases from existing binding identities.

use crate::{HirNode, HirRef, extraction_profile::ExtractionProfile, unit_store::EvidenceFacts};

pub(crate) fn initializer<'a>(
    node: &HirNode,
    facts: &'a dyn EvidenceFacts,
    ty: &crate::data_types::TypeToken,
) -> Result<&'a HirNode, String> {
    let target = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Missing shared-reference binding resolution")?;
    let binding = facts
        .bindings
        .iter()
        .find(|binding| binding.hir == *target)
        .ok_or("Missing shared-reference binding")?;
    if binding.is_param {
        return Err("Reference formal lacks signature correspondence".into());
    }
    if !matches!(ty, crate::data_types::TypeToken::Shared(_))
        || binding.is_mut
        || !ExtractionProfile::from_facts(facts)?.supports("rust.shared-reference-locals")
    {
        return Err("Shared-reference local requires its immutable binding capability".into());
    }
    let value = declaration(&facts.body, target)
        .and_then(|declaration| {
            declaration
                .children
                .iter()
                .find(|child| child.kind.starts_with("expr_"))
        })
        .ok_or("Missing shared-reference local initializer")?;
    if value.span.end > node.span.start {
        return Err("Reference local read must follow its complete initializer".into());
    }

    Ok(value)
}

fn declaration<'a>(node: &'a HirNode, target: &HirRef) -> Option<&'a HirNode> {
    if node.kind == "stmt_let"
        && node
            .children
            .iter()
            .any(|child| child.kind == "pat_binding" && child.hir == *target)
    {
        return Some(node);
    }

    node.children
        .iter()
        .find_map(|child| declaration(child, target))
}
