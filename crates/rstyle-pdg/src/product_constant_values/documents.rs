//! Check the complete typed literal and absence of invented runtime effects.
use super::*;
use crate::{NodeCategory, PdgNode, producer::PairView};
use std::collections::BTreeSet;

pub fn verify_documents(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    verify_document_policy(&pair.pdg.profile, facts)?;
    verify_document_policy(
        &crate::ProfileId {
            id: pair.cfg.profile.name.clone(),
            version: pair.cfg.profile.version.clone(),
        },
        facts,
    )?;

    let sources = crate::field_locations::operations(&facts.body, &["expr_path"]);
    let mut expected = BTreeSet::new();
    for source in sources.into_iter().filter(|source| candidate(source)) {
        let checked = verify(source, facts)?;
        let node =
            crate::data_documents::typed_operation(pair, source, CONSTRUCT, &checked.owner_type)?;
        verify_value(node, &literal(&checked)?)?;
        crate::data_documents::verify_saved_operands(pair, node, &[])?;
        if !expected.insert(node.id.as_str()) {
            return Err("Ambiguous serialized product constant source operation".into());
        }
    }

    let actual: BTreeSet<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some(CONSTRUCT))
        .map(|node| node.id.as_str())
        .collect();
    if actual != expected {
        return Err("Product constant document inventory differs from original uses".into());
    }

    Ok(())
}

fn verify_value(node: &PdgNode, expected: &str) -> Result<(), String> {
    if node.category != NodeCategory::Computation
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(expected)
        || node.attributes.operator.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || !node.attributes.outcomes.is_empty()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Product constant loses its typed value or invents runtime effects".into());
    }
    Ok(())
}
