//! Resolve reference-bearing locals from existing binding identities and flow.

mod flow;
pub(crate) use flow::OwnedIndex;
mod origins;

use crate::{HirNode, HirRef, extraction_profile::ExtractionProfile, unit_store::EvidenceFacts};

pub(crate) fn supported(ty: &crate::data_types::TypeToken, profile: ExtractionProfile) -> bool {
    use crate::data_types::TypeToken;
    (matches!(ty, TypeToken::Option(payload) if matches!(payload.as_ref(), TypeToken::Shared(_)))
        && profile.supports("rust.checked-slice-option"))
        || (matches!(
            ty,
            TypeToken::SliceArrayResult {
                length: 8,
                borrowed: true
            }
        ) && profile.supports("rust.native-borrowed-byte-readers"))
        || (matches!(
            ty,
            TypeToken::SliceArrayResult {
                length: 4,
                borrowed: true
            }
        ) && profile.supports("rust.checked-borrowed-result-flow"))
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
    let borrowed_result =
        crate::extraction_profile::facts_support(facts, "rust.checked-borrowed-result-flow")
            && facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .and_then(|text| crate::data_types::parse(text).ok())
                .is_some_and(|ty| result_flow(&ty, facts));
    if borrowed_result {
        return true;
    }
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

fn result_flow(ty: &crate::data_types::TypeToken, facts: &dyn EvidenceFacts) -> bool {
    if crate::borrowed_result_types::uses_result(ty) {
        return true;
    }
    let crate::data_types::TypeToken::Result { ok, err } = ty else {
        return false;
    };
    [
        (
            crate::concrete_results::copy_errors::CAPABILITY,
            crate::concrete_results::copy_errors::GRAMMAR,
        ),
        (
            crate::concrete_results::fresh_products::CAPABILITY,
            crate::concrete_results::fresh_products::GRAMMAR,
        ),
    ]
    .into_iter()
    .any(|(capability, grammar)| {
        crate::extraction_profile::facts_support(facts, capability) && grammar.permits(ok, err)
    })
}

pub(crate) fn selected(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<crate::data_origins::Origin>, String> {
    facts.reference_origins(node, selectors)
}

pub(crate) fn compute_origins(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<crate::data_origins::Origin>, String> {
    origins::selected(node, selectors, facts)
}

pub(crate) fn indexed_origins(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
    index: &OwnedIndex,
) -> Result<Vec<crate::data_origins::Origin>, String> {
    origins::selected_with(node, selectors, facts, index)
}
