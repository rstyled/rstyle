use super::*;
use serde_json::json;

fn pair() -> Pair {
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
