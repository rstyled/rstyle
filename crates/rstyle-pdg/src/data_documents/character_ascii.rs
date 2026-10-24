//! Independent scalar receiver/value and pinned ASCII summary correspondence.
use super::*;
use crate::{HirNode, character_ascii, unit_store::EvidenceFacts};

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    calls: &[&HirNode],
) -> Result<(), String> {
    let sources: Vec<_> = calls
        .iter()
        .copied()
        .filter(|source| character_ascii::candidate(source, facts))
        .collect();
    verify_construct_inventory(pair, [character_ascii::CONSTRUCT], [sources.len()])?;
    for source in sources {
        let node = serialized_operation(pair, &source.span, |node| {
            node.category == NodeCategory::Call
        })?;
        if node.attributes.construct.as_deref() != Some(character_ascii::CONSTRUCT)
            || node.original_kind.known_value() != Some("expr_method_call")
            || node.attributes.call_form != Some(crate::CallForm::Direct)
            || node
                .annotations
                .get("compat.rust.hir-r0:type")
                .and_then(|value| value.get("value"))
                .and_then(serde_json::Value::as_str)
                != Some("bool")
            || !node.attributes.reads.is_empty()
            || !node.attributes.writes.is_empty()
            || !node.attributes.defines.is_empty()
            || !node.attributes.symbol_ids.is_empty()
            || node.attributes.literal.is_some()
            || node.annotations.contains_key(crate::data_places::KEY)
        {
            return Err(
                "Character ASCII loses its scalar call/value semantics or invents effects".into(),
            );
        }
        summary_records::Record {
            source,
            node,
            summary: character_ascii::resolve(source, facts)?
                .ok_or("Missing character ASCII summary")?,
        }
        .verify(pair, facts)?;
    }
    Ok(())
}
