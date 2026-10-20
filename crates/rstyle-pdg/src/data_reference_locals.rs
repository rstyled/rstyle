//! Resolve reference-bearing locals from existing binding identities and flow.

mod flow;
mod origins;

use crate::{HirNode, HirRef, extraction_profile::ExtractionProfile, unit_store::EvidenceFacts};

pub(crate) fn supported(ty: &crate::data_types::TypeToken, profile: ExtractionProfile) -> bool {
    use crate::data_types::TypeToken;
    (matches!(
        ty,
        TypeToken::SliceArrayResult {
            length: 8,
            borrowed: true
        }
    ) && profile.supports("rust.native-borrowed-byte-readers"))
        || (matches!(ty, TypeToken::Shared(_)) && profile.supports("rust.shared-reference-locals"))
        || (matches!(ty, TypeToken::Tuple(_) | TypeToken::Struct { .. })
            && profile.supports("rust.reference-bearing-values")
            && crate::data_origins::shared(ty))
}

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
        .ok_or("Missing reference-bearing binding resolution")?;
    let binding = facts
        .bindings
        .iter()
        .find(|binding| binding.hir == *target)
        .ok_or("Missing reference-bearing binding")?;
    if binding.is_param {
        return Err("Reference formal lacks signature correspondence".into());
    }
    if binding.is_mut || !supported(ty, ExtractionProfile::from_facts(facts)?) {
        return Err("Reference-bearing local requires its immutable binding capability".into());
    }
    let value = declaration(&facts.body, target)
        .and_then(|declaration| {
            declaration
                .children
                .iter()
                .find(|child| child.kind.starts_with("expr_"))
        })
        .ok_or("Missing reference-bearing local initializer")?;
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

pub(crate) fn flow_enabled(facts: &dyn EvidenceFacts) -> bool {
    ExtractionProfile::from_facts(facts)
        .is_ok_and(|profile| profile.supports("rust.reference-rebindings"))
        && facts.bindings.iter().any(|binding| {
            binding.is_mut
                && binding
                    .ty
                    .normalized
                    .as_deref()
                    .and_then(|token| crate::data_types::parse(token).ok())
                    .is_some_and(|ty| crate::data_origins::shared(&ty))
        })
}

pub(crate) fn selected(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<crate::data_origins::Origin>, String> {
    origins::selected(node, selectors, facts)
}
