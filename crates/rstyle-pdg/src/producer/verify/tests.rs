use super::*;
use serde_json::json;

pub(super) fn pair() -> Pair {
    let mut pdg: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor-compat/pdg.example.json"
    )))
    .unwrap();
    for edge in pdg["edges"].as_array_mut().unwrap() {
        edge["policy"] = json!(crate::ProfileId::hir_r0());
    }
    let conformance = pdg["extraction_evidence"]["conformance"].take();
    pdg["extraction_evidence"]["conformance"] = json!([{
        "profile": crate::ProfileId::hir_r0(), "status": conformance["status"],
        "reasons": conformance["reasons"]
    }]);
    pdg["function"]["reference_count"]["policy"] = json!(crate::ProfileId::hir_r0());
    Pair {
        pdg: serde_json::from_value(pdg).unwrap(),
        cfg: serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vendor-compat/cfg.example.json"
        )))
        .unwrap(),
    }
}

#[test]
fn exact_comparison_preserves_semantics_and_conformance_profile_inventory() {
    let mut original = pair();
    original.pdg.extraction_evidence.conformance[0].status = ConformanceStatus::Conformant;
    original.pdg.extraction_evidence.conformance[0].reasons = vec!["informational".into()];
    compare_documents(original.as_view(), &mut pair(), &BTreeMap::new()).unwrap();
    for mutation in ["profile", "inventory", "pdg", "cfg"] {
        let mut changed = pair();
        match mutation {
            "profile" => {
                changed.pdg.extraction_evidence.conformance[0]
                    .profile
                    .version = "unknown".into()
            }
            "inventory" => changed.pdg.extraction_evidence.conformance.clear(),
            "pdg" => changed.pdg.nodes[0].attributes.construct = Some("forged".into()),
            "cfg" => changed.cfg.entry_node_id = "forged".into(),
            _ => unreachable!(),
        }
        assert!(
            compare_documents(changed.as_view(), &mut pair(), &BTreeMap::new()).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn reordered_or_renamed_documents_still_use_complete_normalized_comparison() {
    let mut changed = pair();
    changed.pdg.nodes.reverse();
    changed.cfg.nodes.reverse();
    compare_documents(changed.as_view(), &mut pair(), &BTreeMap::new()).unwrap();
    let original = changed.pdg.nodes[0].id.clone();
    changed.pdg.nodes[0].id = "renamed".into();
    let names = BTreeMap::from([("renamed".into(), original.clone())]);
    compare_documents(changed.as_view(), &mut pair(), &names).unwrap();
    changed.pdg.nodes[0].attributes.construct = Some("forged".into());
    assert!(compare_documents(changed.as_view(), &mut pair(), &names).is_err());
    let names = BTreeMap::from([(original, "different".into())]);
    assert!(compare_documents(pair().as_view(), &mut pair(), &names).is_err());
}

#[test]
fn correspondence_distinguishes_roles_at_the_same_source_operation() {
    let mut expected = pair();
    let mut first = expected.pdg.nodes[0].clone();
    first.id = "none".into();
    first.attributes.construct = Some("try.option-none".into());
    let mut second = first.clone();
    second.id = "payload".into();
    second.attributes.construct = Some("try.option-some-reference".into());
    expected.pdg.nodes = vec![first, second];
    let mut actual = pair();
    actual.pdg.nodes = expected.pdg.nodes.iter().rev().cloned().collect();
    actual.pdg.nodes[0].id = "renamed-payload".into();
    let names = node_names(actual.as_view(), expected.as_view()).unwrap();
    assert_eq!(names["renamed-payload"], "payload");
    assert_eq!(names["none"], "none");
    actual.pdg.nodes[0].attributes.construct = Some("try.option-none".into());
    assert!(node_names(actual.as_view(), expected.as_view()).is_err());
}

#[test]
fn correspondence_preserves_every_original_operation_discriminator() {
    let expected = pair();
    for mutation in [
        "category",
        "construct",
        "span",
        "kind",
        "status",
        "reason",
        "position",
        "operation",
    ] {
        let mut actual = pair();
        let node = &mut actual.pdg.nodes[0];
        mutate_operation(node, mutation);
        assert!(
            node_names(actual.as_view(), expected.as_view()).is_err(),
            "{mutation}"
        );
    }
}

fn mutate_operation(node: &mut PdgNode, mutation: &str) {
    match mutation {
        "category" => {
            node.category = if node.category == NodeCategory::Other {
                NodeCategory::Read
            } else {
                NodeCategory::Other
            }
        }
        "construct" => node.attributes.construct = Some("changed".into()),
        "span" => {
            node.source_span = Some(Span {
                source_id: "source".into(),
                start: 9999,
                end: 10000,
            })
        }
        "kind" => node.original_kind.value = Some("changed".into()),
        "status" => {
            node.original_kind = StringFact {
                status: FactStatus::Unknown,
                value: None,
                reason: Some("changed".into()),
            }
        }
        "reason" => node.original_kind.reason = Some("changed".into()),
        "position" => node.attributes.position = Some(9999),
        "operation" => {
            node.attributes.extensions.insert(
                "compat.rust.hir-r0:source-operation".into(),
                json!({"role":"changed"}),
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn correspondence_rejects_ambiguous_candidates_and_reused_operations() {
    let mut expected = pair();
    let first = expected.pdg.nodes[0].clone();
    let mut second = first.clone();
    second.id = "second".into();
    expected.pdg.nodes = vec![first.clone(), second];
    assert!(node_names(expected.as_view(), expected.as_view()).is_err());
    expected.pdg.nodes[1].attributes.position = Some(9999);
    let mut actual = pair();
    actual.pdg.nodes = vec![first.clone(), first];
    actual.pdg.nodes[1].id = "second-claim".into();
    assert_eq!(
        node_names(actual.as_view(), expected.as_view()).unwrap_err(),
        vec!["Two nodes claim the same source operation"]
    );
}

#[test]
fn large_reversed_correspondence_keeps_exact_renamed_roles() {
    let mut expected = pair();
    let template = expected.pdg.nodes[0].clone();
    expected.pdg.nodes = (0..4096)
        .map(|position| {
            let mut node = template.clone();
            node.id = format!("expected-{position}");
            node.attributes.position = Some(position);
            node
        })
        .collect();
    let mut actual = pair();
    actual.pdg.nodes = expected
        .pdg
        .nodes
        .iter()
        .rev()
        .cloned()
        .map(|mut node| {
            node.id = format!("actual-{}", node.attributes.position.unwrap());
            node
        })
        .collect();
    let names = node_names(actual.as_view(), expected.as_view()).unwrap();
    assert_eq!(names.len(), 4096);
    for position in 0..4096 {
        assert_eq!(
            names[&format!("actual-{position}")],
            format!("expected-{position}")
        );
    }
}

#[test]
fn correspondence_retains_overwritten_name_behavior_before_boundary_validation() {
    let mut expected = pair();
    let template = expected.pdg.nodes[0].clone();
    expected.pdg.nodes = (0..3)
        .map(|position| {
            let mut node = template.clone();
            node.id = format!("expected-{position}");
            node.attributes.position = Some(position);
            node
        })
        .collect();
    let mut actual = pair();
    actual.pdg.nodes = vec![
        expected.pdg.nodes[0].clone(),
        expected.pdg.nodes[1].clone(),
        expected.pdg.nodes[0].clone(),
    ];
    actual.pdg.nodes[0].id = "overwritten".into();
    actual.pdg.nodes[1].id = "overwritten".into();
    actual.pdg.nodes[2].id = "last".into();
    assert_eq!(
        node_names(actual.as_view(), expected.as_view()).unwrap(),
        BTreeMap::from([
            ("overwritten".into(), "expected-1".into()),
            ("last".into(), "expected-0".into()),
        ])
    );
}

#[test]
fn correspondence_keeps_absent_null_and_nested_operation_evidence_distinct() {
    let mut expected = pair();
    expected.pdg.nodes.truncate(1);
    let key = "compat.rust.hir-r0:source-operation";
    expected.pdg.nodes[0]
        .attributes
        .extensions
        .insert(key.into(), Value::Null);
    let mut actual = pair();
    actual.pdg.nodes = expected.pdg.nodes.clone();
    assert!(node_names(actual.as_view(), expected.as_view()).is_ok());
    actual.pdg.nodes[0].attributes.extensions.remove(key);
    assert!(node_names(actual.as_view(), expected.as_view()).is_err());
    let evidence = json!({"operands":[{"position":1,"saved":true}]});
    expected.pdg.nodes[0]
        .attributes
        .extensions
        .insert(key.into(), evidence.clone());
    actual.pdg.nodes[0]
        .attributes
        .extensions
        .insert(key.into(), evidence);
    assert!(node_names(actual.as_view(), expected.as_view()).is_ok());
    actual.pdg.nodes[0]
        .attributes
        .extensions
        .get_mut(key)
        .unwrap()["operands"][0]["saved"] = json!(false);
    assert!(node_names(actual.as_view(), expected.as_view()).is_err());
}
