//! Byte values and selectors checked directly from original source and documents.

use super::*;
use crate::{HirNode, byte_literals, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    for source in crate::field_locations::operations(&facts.body, &["expr_lit", "expr_match"]) {
        if source.kind == "expr_match" {
            verify_match(pair, source, facts)?;
        } else if byte_literals::uses(source, facts) {
            policy(pair)?;
            byte_literals::verify_node(source, facts)?;
            let node = super::value_operation(pair, source)?;
            super::aggregates::check_value(node, source)?;
            effects(node, NodeCategory::Computation)?;
            literal(node, source.lit.as_deref())?;
            if node.attributes.construct.is_some() {
                return Err("Byte literal invents a composite operation".into());
            }
            super::verify_saved_operands(pair, node, &[])?;
        }
    }
    Ok(())
}

fn policy(pair: PairView<'_>) -> Result<(), String> {
    if !crate::extraction_profile::policy_supports(&pair.pdg.profile, byte_literals::CAPABILITY) {
        return Err("Byte scalars require their registered document policy".into());
    }
    Ok(())
}

fn verify_match(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let Some((scrutinee, arms)) = source.children.split_first() else {
        return Err("Byte match loses its scrutinee".into());
    };
    for pattern in arms.iter().filter_map(|arm| arm.children.first()) {
        if !byte_literals::uses(pattern, facts) {
            continue;
        }
        policy(pair)?;
        let mut leaves = Vec::new();
        collect(pattern, facts, &mut leaves)?;
        let (construct, expected) = selector(pattern, &leaves)?;
        let node = super::typed_operation(pair, pattern, construct, "bool")?;
        effects(node, NodeCategory::Control)?;
        literal(node, expected.as_deref())?;
        super::verify_saved_operands(pair, node, &[super::value_operation(pair, scrutinee)?])?;
    }
    Ok(())
}

fn collect<'a>(
    pattern: &'a HirNode,
    facts: &dyn EvidenceFacts,
    leaves: &mut Vec<&'a HirNode>,
) -> Result<(), String> {
    byte_literals::verify_node(pattern, facts)?;
    if pattern.kind == "pat_or" {
        for child in &pattern.children {
            collect(child, facts, leaves)?;
        }
    } else {
        if pattern.kind == "pat_range" {
            let range = crate::scalar_patterns::IntegerRange::from_byte_pattern(
                byte_literals::source(pattern, facts).ok_or("Missing original byte range")?,
                pattern,
                facts.build.pointer_width.value,
                true,
            )?;
            if pattern.lit.as_deref() != Some(&range.text()) {
                return Err("Byte range loses original bounds or inclusivity".into());
            }
        }
        leaves.push(pattern);
    }
    Ok(())
}

fn selector(
    pattern: &HirNode,
    leaves: &[&HirNode],
) -> Result<(&'static str, Option<String>), String> {
    let (construct, literal) = match pattern.kind.as_str() {
        "pat_literal" => ("match.literal", pattern.lit.clone()),
        "pat_range" => ("match.range", pattern.lit.clone()),
        "pat_or" if leaves.iter().all(|leaf| leaf.kind == "pat_literal") => (
            "match.or",
            Some(
                serde_json::to_string(&leaves.iter().map(|leaf| &leaf.lit).collect::<Vec<_>>())
                    .map_err(|e| e.to_string())?,
            ),
        ),
        "pat_or" => (
            "match.or-patterns",
            Some(
                serde_json::to_string(
                    &leaves
                        .iter()
                        .map(|leaf| (&leaf.kind, &leaf.lit))
                        .collect::<Vec<_>>(),
                )
                .map_err(|e| e.to_string())?,
            ),
        ),
        _ => return Err("Byte selector has an unsupported pattern".into()),
    };
    Ok((construct, literal))
}

fn literal(node: &PdgNode, expected: Option<&str>) -> Result<(), String> {
    if node
        .attributes
        .literal
        .as_ref()
        .and_then(|value| value.known_value())
        != expected
    {
        return Err("Serialized byte value/selector disagrees with original source".into());
    }
    Ok(())
}

fn effects(node: &PdgNode, category: NodeCategory) -> Result<(), String> {
    super::verify_value_effects(node, true, "Byte value/selector invents effects")?;
    if node.category != category
        || node.annotations.contains_key(crate::data_places::KEY)
        || node.attributes.outcomes
            != if category == NodeCategory::Control {
                vec!["true", "false"]
            } else {
                Vec::new()
            }
    {
        return Err("Byte value/selector invents a place or changes category".into());
    }
    Ok(())
}
