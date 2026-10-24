//! Independently check value projections without inventing memory locations.

use crate::{HirNode, NodeCategory, PdgNode, producer::PairView, unit_store::EvidenceFacts};

pub(super) fn verify(
    node: &PdgNode,
    source: &HirNode,
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let index = crate::data_value_fields::index(source, facts)?.to_string();
    super::verify_value_effects(
        node,
        true,
        "Value projection invents memory or scalar effects",
    )?;
    if node.category != NodeCategory::Computation
        || node.attributes.construct.as_deref() != Some("data.value-field")
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(|literal| literal.known_value())
            != Some(index.as_str())
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Serialized value projection loses its exact component identity".into());
    }
    let edges: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == node.id && edge.subkind == "value_operand")
        .collect();
    let [edge] = edges.as_slice() else {
        return if edges.is_empty() && !super::live_operations(pair)?.contains(node.id.as_str()) {
            Ok(())
        } else {
            Err("Value projection requires one evaluated aggregate operand".into())
        };
    };
    if edge.operand_position != Some(0) {
        return Err("Value projection base position disagrees".into());
    }
    let value = pair
        .pdg
        .nodes
        .iter()
        .find(|value| value.id == edge.source)
        .ok_or("Missing evaluated aggregate projection base")?;

    super::aggregates::check_value(value, &source.children[0])
}
