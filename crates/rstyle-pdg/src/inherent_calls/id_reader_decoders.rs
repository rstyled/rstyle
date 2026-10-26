//! Original ID reader calls join checked generated members to their emitted impl.
use super::*;
use crate::unit_store::EvidenceFacts;

pub(super) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    resolution: &serde_json::Value,
) -> Result<bool, String> {
    if !caller
        .body
        .any(&|node| crate::id_error_maps::candidate(node, caller))
    {
        return Ok(false);
    }
    let callee_resolution = callee.nominal_resolution()?;
    let Some(source) = callee_resolution.transport_source() else {
        return Ok(false);
    };
    source.verify(callee.facts())?;
    let member = source.member();
    let capture = callee
        .generated_source_capture()?
        .ok_or("Missing generated ID decoder emitted roster")?;
    let emitted = capture
        .emitted
        .iter()
        .filter(|entry| entry.definition == *member.owner())
        .collect::<Vec<_>>();
    let [emitted] = emitted.as_slice() else {
        return Err("ID decoder lacks a unique emitted method".into());
    };
    let observed: CallOwner =
        serde_json::from_value(resolution["inherent"].clone()).map_err(|e| e.to_string())?;
    let nominal = source.request().copy_requests()[1]
        .nominal_identity()
        .ok_or("ID decoder lacks its complete nominal owner")?;
    if node.kind != "expr_call"
        || emitted.kind != "AssocFn"
        || observed.identity != IDENTITY
        || observed.implementation_identity != emitted.parent.definition_identity
        || resolution["definition_identity"].as_str()
            != Some(member.owner().definition_identity.as_str())
        || resolution["path"].as_str() != Some(member.owner().path.as_str())
        || member.function().sig.ident != "from_ne_bytes"
        || callee.signature.has_self
        || !resolved_expression(node, caller, Some(callee), resolution)?
        || !ufcs_owner_matches(node, caller, nominal)?
    {
        return Err("ID decoder selected call differs from its checked generated owner".into());
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_decoder_name_and_plain_call_must_match_the_selected_definition() {
        let mut facts: HirFnFacts = serde_json::from_str(include_str!(
            "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        let selected = serde_json::json!({"path": "generated::FirstID::from_ne_bytes"});
        for (source, expected) in [
            ("FirstID::from_ne_bytes(bytes)", true),
            ("FirstID::from_le_bytes(bytes)", false),
            ("FirstID::other(bytes)", false),
            ("FirstID::from_ne_bytes::<u32>(bytes)", false),
            ("<FirstID as Other>::from_ne_bytes(bytes)", false),
        ] {
            facts.source.bytes = source.into();
            let mut node = facts.body.clone();
            node.kind = "expr_call".into();
            node.span = ByteSpan {
                start: 0,
                end: source.len() as u32,
            };
            assert_eq!(
                resolved_expression(&node, &facts, None, &selected).unwrap(),
                expected,
                "{source}"
            );
        }
    }
}
