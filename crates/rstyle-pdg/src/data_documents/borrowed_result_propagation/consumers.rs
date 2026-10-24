//! Complete outgoing data ownership uses, including alternate and storage edges.
use super::*;
use crate::{EdgeFamily, PdgEdge};

pub(in crate::data_documents) fn verify<const N: usize>(
    pair: PairView<'_>,
    saved: &PdgNode,
    operations: [&PdgNode; N],
) -> Result<(), String> {
    let edges: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.source == saved.id && edge.kind == EdgeFamily::Data)
        .collect();
    let expected: Vec<_> = operations.iter().map(|node| node.id.as_str()).collect();
    exact_uses(&edges, &expected)
}

fn exact_uses(edges: &[&PdgEdge], expected: &[&str]) -> Result<(), String> {
    if edges.len() != expected.len()
        || edges.iter().any(|edge| {
            edge.subkind != "value_operand"
                || edge.operand_position != Some(0)
                || edge.symbol_id.is_some()
                || edge.definition_id.is_some()
                || edge.location_id.is_some()
                || edge.outcome.is_some()
                || edge.loop_carried.is_some()
        })
        || expected
            .iter()
            .any(|target| edges.iter().filter(|edge| edge.target == *target).count() != 1)
    {
        return Err("Saved borrowed Result has an unproved duplicate, alternate, storage or conditional ownership use".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn operand(target: &str) -> PdgEdge {
        serde_json::from_value(json!({
            "policy":{"id":"test","version":"1"},"id":"edge","source":"saved","target":target,
            "kind":"data","subkind":"value_operand","operand_position":0,
            "evidence":{"policy":{"id":"test","version":"1"},"status":"supported","description":"test"},
            "provenance":{"policy":{"id":"test","version":"1"},"origin":"analysis","description":"test"}
        })).unwrap()
    }

    #[test]
    fn rejects_alternate_storage_and_unregistered_ownership_edges() {
        let selector = operand("selector");
        let error = operand("error");
        let success = operand("success");
        let expected = ["selector", "error", "success"];
        exact_uses(&[&selector, &error, &success], &expected).unwrap();
        for subkind in [
            "value_alternative",
            "reaching_definition",
            "invented_data_use",
        ] {
            let mut attack = operand("extra");
            attack.subkind = subkind.into();
            assert!(exact_uses(&[&selector, &error, &success, &attack], &expected).is_err());
            attack.target = "success".into();
            assert!(exact_uses(&[&selector, &error, &attack], &expected).is_err());
        }
    }

    #[test]
    fn rewrapped_error_has_exactly_one_whole_value_return_use() {
        let returned = operand("return");
        exact_uses(&[&returned], &["return"]).unwrap();
        assert!(exact_uses(&[], &["return"]).is_err());
        assert!(exact_uses(&[&returned, &returned], &["return"]).is_err());
        for field in [
            "operand_position",
            "symbol_id",
            "definition_id",
            "location_id",
            "outcome",
            "loop_carried",
        ] {
            let mut wire = serde_json::to_value(&returned).unwrap();
            wire[field] = match field {
                "operand_position" => json!(1),
                "loop_carried" => json!(false),
                _ => json!("invented"),
            };
            let attack = serde_json::from_value(wire).unwrap();
            assert!(exact_uses(&[&attack], &["return"]).is_err(), "{field}");
        }
    }
}
