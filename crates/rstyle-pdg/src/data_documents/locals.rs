//! Independently check whole local values and serialized reaching definitions.

use crate::{BindingFact, NodeCategory, PdgNode, producer::PairView, unit_store::EvidenceFacts};
use std::collections::BTreeMap;

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let profile = crate::extraction_profile::ExtractionProfile::from_facts(facts)?;
    let bindings: BTreeMap<_, _> = facts
        .bindings
        .iter()
        .filter(|binding| {
            (!binding.is_param || binding.is_mut)
                && binding.ty.normalized.as_deref().is_some_and(|token| {
                    !crate::concrete_types::scalar(token)
                        && crate::data_locals::supported(token, profile)
                })
        })
        .map(|binding| (binding.symbol.as_str(), binding))
        .collect();
    if bindings.is_empty() {
        return Ok(());
    }

    for node in &pair.pdg.nodes {
        if let Some(binding) = source_binding(node, facts)?
            && bindings.contains_key(binding.symbol.as_str())
        {
            verify_access(node, binding)?;
            if node.category != NodeCategory::Read {
                verify_value(node, binding, pair, facts)?;
            }
        }
    }
    crate::cyclic_values::verify_bindings(pair.pdg, pair.cfg).map_err(|errors| errors.join("; "))
}

fn source_binding<'a>(
    node: &PdgNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Option<&'a BindingFact>, String> {
    if !matches!(
        node.category,
        NodeCategory::Read | NodeCategory::Declaration | NodeCategory::Assignment
    ) {
        return Ok(None);
    }
    let source = super::origins::source_node(node, facts)?;
    if node.category == NodeCategory::Declaration {
        let symbol = source
            .children
            .iter()
            .find(|child| child.kind == "pat_binding")
            .and_then(|pattern| pattern.binding.as_deref());
        return Ok(facts
            .bindings
            .iter()
            .find(|binding| Some(binding.symbol.as_str()) == symbol));
    }
    let source = if node.category == NodeCategory::Assignment {
        source
            .children
            .first()
            .ok_or("Missing local assignment target")?
    } else {
        source
    };
    let target = source
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .and_then(|resolution| resolution.target.as_ref());

    Ok(facts
        .bindings
        .iter()
        .find(|binding| Some(&binding.hir) == target))
}

fn verify_access(node: &PdgNode, binding: &BindingFact) -> Result<(), String> {
    let accesses = if node.category == NodeCategory::Read {
        &node.attributes.reads
    } else {
        &node.attributes.writes
    };
    if node.attributes.symbol_ids != [binding.symbol.clone()]
        || accesses.len() != 1
        || accesses[0].symbol_id.as_ref() != Some(&binding.symbol)
        || accesses[0].location_id.is_some()
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(|fact| fact.known_value())
            != (node.category == NodeCategory::Assignment).then_some("=")
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
    {
        return Err("Local aggregate operation loses its exact source binding".into());
    }
    let expected = if node.category == NodeCategory::Read {
        binding
            .ty
            .normalized
            .as_deref()
            .ok_or("Missing local aggregate type")?
    } else {
        "unit"
    };

    if super::origins::wire_type(node)? != expected {
        return Err("Local aggregate operation type disagrees with its source role".into());
    }
    Ok(())
}

fn verify_value(
    node: &PdgNode,
    binding: &BindingFact,
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
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
            Err("Local aggregate definition requires one whole value".into())
        };
    };
    let value = pair
        .pdg
        .nodes
        .iter()
        .find(|value| value.id == edge.source)
        .ok_or("Missing local aggregate definition value")?;

    if edge.operand_position != Some(0)
        || Some(super::origins::wire_type(value)?.as_str()) != binding.ty.normalized.as_deref()
    {
        return Err("Local aggregate definition loses its typed whole-value operand".into());
    }
    let source = super::origins::source_node(node, facts)?;
    let initializer = if node.category == NodeCategory::Declaration {
        source
            .children
            .iter()
            .find(|child| child.kind.starts_with("expr_"))
    } else {
        source.children.get(1)
    }
    .ok_or("Missing original local definition value")?;
    super::aggregates::check_value(value, initializer)?;

    Ok(())
}
