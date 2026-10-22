//! Recompute fieldless values/casts from complete original source and type proof.
use super::*;
use crate::{HirNode, fieldless_enums as semantics, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let Inventory { values, casts } = inventory(facts)?;
    super::verify_construct_inventory(
        pair,
        [semantics::VALUE, semantics::CAST],
        [values.len(), casts.len()],
    )?;
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        semantics::CAPABILITY,
        || !values.is_empty() || !casts.is_empty(),
        "Fieldless enum operations require their registered document policy",
    )?;
    for source in values {
        let (tag, value) = semantics::value(source, facts)?;
        let node = operation(pair, source, semantics::VALUE, &tag)?;
        if node
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(value.to_string().as_str())
        {
            return Err("Fieldless value loses its independently checked discriminant".into());
        }
        super::verify_saved_operands(pair, node, &[])?;
    }

    for source in casts {
        semantics::cast(source, facts)?;
        let node = operation(pair, source, semantics::CAST, "as")?;
        if node.attributes.literal.is_some() {
            return Err("Fieldless cast invents a constant result".into());
        }
        super::verify_saved_operands(
            pair,
            node,
            &[super::value_operation(pair, &source.children[0])?],
        )?;
    }

    Ok(())
}

struct Inventory<'a> {
    values: Vec<&'a HirNode>,
    casts: Vec<&'a HirNode>,
}

fn inventory(facts: &dyn EvidenceFacts) -> Result<Inventory<'_>, String> {
    let mut operations = Vec::new();
    collect_operations(&facts.body, facts, &mut operations)?;
    let values: Vec<_> = operations
        .iter()
        .copied()
        .filter(|node| semantics::value_candidate(node, facts))
        .collect();
    let casts: Vec<_> = operations
        .iter()
        .copied()
        .filter(|node| semantics::cast_candidate(node, facts))
        .collect();

    Ok(Inventory { values, casts })
}

/// Checked structural boundaries retain every evaluated original operand.
/// Generated Try arms and fresh mixed-enum errors have their own exact inventory.
fn collect_operations<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
    operations: &mut Vec<&'a HirNode>,
) -> Result<(), String> {
    if let Some(operands) = crate::data_frontend::borrowed_results::operands(node, facts) {
        for operand in operands? {
            collect_operations(operand, facts, operations)?;
        }
        return Ok(());
    }

    for child in &node.children {
        collect_operations(child, facts, operations)?;
    }
    operations.push(node);
    Ok(())
}

fn operation<'a>(
    pair: PairView<'a>,
    source: &HirNode,
    construct: &str,
    operator: &str,
) -> Result<&'a PdgNode, String> {
    let node = super::typed_operation(
        pair,
        source,
        construct,
        crate::data_aggregates::token(source)?,
    )?;
    if node.category != NodeCategory::Computation
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(operator)
        || node.attributes.callee.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Fieldless operation loses exact semantics or invents effects".into());
    }

    Ok(node)
}
