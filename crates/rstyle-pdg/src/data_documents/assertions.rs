//! Independently check message/capture semantics, source inventory and saved values.
use crate::{HirNode, NodeCategory, PdgNode, producer::PairView, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = crate::field_locations::operations(&facts.body, &["expr_assert"]);
    let mut active = 0;
    let mut comparisons = 0;

    for source in sources {
        let assertion = crate::assertions::Assertion::parse(&source.snippet)?;
        if assertion.right.is_some() {
            comparisons += verify_comparison(pair, facts, source)?;
        } else if let Some(diagnostic) = assertion.diagnostic {
            crate::assertions::formatting::verify(source, &source.snippet, &diagnostic)?;
            if !crate::extraction_profile::policy_supports(&pair.pdg.profile, "rust.end-of-input")
                || crate::data_library::registry_for(facts).is_none()
            {
                return Err(
                    "Formatted scalar assertion requires its registered build/policy".into(),
                );
            }
            if source.lit.as_deref() == Some("true") {
                verify_active(pair, source, &diagnostic)?;
                active += 1;
            }
        }
    }

    verify_inventory(pair, active, comparisons)
}

fn verify_inventory(pair: PairView<'_>, active: usize, comparisons: usize) -> Result<(), String> {
    let actual = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.attributes
                .construct
                .as_deref()
                .is_some_and(|name| name.ends_with(".scalar-format"))
        })
        .count();

    let actual_comparisons = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.attributes.construct.as_deref(),
                Some("core::assert_eq" | "core::debug_assert_eq")
            )
        })
        .count();
    if actual_comparisons != comparisons {
        return Err("Comparison assertion inventory disagrees with complete source".into());
    }
    if actual != active {
        return Err("Formatted assertion inventory disagrees with complete source".into());
    }
    Ok(())
}

fn verify_active(
    pair: PairView<'_>,
    source: &HirNode,
    diagnostic: &crate::assertions::formatting::Diagnostic,
) -> Result<(), String> {
    let node = super::serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Computation
    })?;
    let construct = format!(
        "{}.scalar-format",
        source
            .res
            .as_ref()
            .ok_or("Missing assertion resolution")?
            .description
    );
    if node.attributes.construct.as_deref() != Some(&construct)
        || node.original_kind.known_value() != Some("expr_assert")
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some("assert")
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(&diagnostic.message)
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
    {
        return Err("Formatted assertion loses its message or builtin scalar semantics".into());
    }
    operands(pair, node, source)
}

fn operands(pair: PairView<'_>, node: &PdgNode, source: &HirNode) -> Result<(), String> {
    let edges: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .collect();
    if edges.len() != source.children.len() {
        return Err("Assertion loses exact condition/diagnostic operand count".into());
    }
    for (position, child) in source.children.iter().enumerate() {
        let found: Vec<_> = edges
            .iter()
            .filter(|edge| edge.operand_position == Some(position as u64))
            .collect();
        let [edge] = found.as_slice() else {
            return Err("Assertion loses ordered diagnostic capture".into());
        };
        crate::helper_calls::verify_operand(pair, &edge.source, child)?;
    }
    Ok(())
}

fn verify_comparison(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    source: &HirNode,
) -> Result<usize, String> {
    if !crate::extraction_profile::policy_supports(&pair.pdg.profile, "rust.accelerator-packing")
        || crate::data_library::registry_for(facts).is_none()
    {
        return Err("Scalar comparison assertion requires its registered build/policy".into());
    }

    crate::assertions::verify(
        source,
        &source.snippet,
        facts.build.debug_assertions.value,
        crate::extraction_profile::ExtractionProfile::from_facts(facts)?,
    )?;
    if source.lit.as_deref() != Some("true") {
        return Ok(0);
    }

    let node = super::serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Computation
    })?;
    if node.attributes.construct.as_deref()
        != source.res.as_ref().map(|res| res.description.as_str())
        || node.original_kind.known_value() != Some("expr_assert")
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some("assert-eq")
        || node.attributes.literal.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
    {
        return Err("Comparison assertion loses builtin equality or exact effects".into());
    }

    operands(pair, node, source)?;
    Ok(1)
}
