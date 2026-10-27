//! Whole aggregate transitions preserve reference components under affine proof.
use crate::{HirNode, data_origins::Origin, unit_store::EvidenceFacts};

pub(crate) const UPDATE: &str = "data.owned-product-update";

pub(crate) fn source(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Option<Vec<Origin>>, String> {
    if !binding_candidate(node, facts) || !crate::owned_product_locals::candidate(facts) {
        return Ok(None);
    }

    let checked = crate::owned_product_locals::verify(facts)?;
    let owned = node
        .res
        .as_ref()
        .and_then(|res| res.target.as_ref())
        .is_some_and(|target| checked.owns_binding(target));
    let update = checked
        .updates()
        .iter()
        .any(|update| update.assignment().hir == node.hir);
    if !owned && !update {
        return Ok(None);
    }

    crate::helper_component_outputs::map(
        checked.initializer_operand(),
        selectors,
        facts,
        |position| {
            let operand = checked
                .initializer_operand()
                .children
                .get(position as usize)
                .ok_or("Owned initializer lacks the exact helper argument")?;
            crate::data_component_origins::source(operand, &[], facts)
        },
    )
    .map(Some)
}

fn binding_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    let receiver = match node.kind.as_str() {
        "expr_path" => node,
        "expr_assign" => match node
            .children
            .first()
            .filter(|target| target.kind == "expr_field")
            .and_then(|target| target.children.first())
        {
            Some(receiver) if receiver.kind == "expr_path" => receiver,
            _ => return false,
        },
        _ => return false,
    };
    let target = receiver
        .res
        .as_ref()
        .filter(|res| res.kind == "local")
        .and_then(|res| res.target.as_ref());
    facts
        .bindings
        .iter()
        .any(|binding| !binding.is_param && binding.is_mut && Some(&binding.hir) == target)
}
