use super::*;
use crate::nominal_boundary::census::{Census, KEY};
use serde_json::{Value, json};
use std::sync::Arc;

fn roster() -> Vec<StyleFnFacts> {
    super::super::tests::roster()
        .into_iter()
        .map(|facts| HirFnFacts {
            name: facts.name,
            fn_span: facts.fn_span,
            source: facts.source,
            signature: facts.signature,
            bindings: Uncaptured::NotRequested,
            body: facts.body.hir,
            build: facts.build,
            source_diagnostics: facts.source_diagnostics,
            source_scan_complete: facts.source_scan_complete,
        })
        .collect()
}

fn encoded() -> (Vec<StyleFnFacts>, Value) {
    let facts = roster();
    let mut bytes = Vec::new();
    let manifest = write_sealed_style_snapshot(&mut bytes, "root-style", &facts).unwrap();
    manifest.verify(&bytes).unwrap();
    (facts, serde_json::from_slice(&bytes).unwrap())
}

fn read(wire: &Value) -> Result<Vec<StyleFnFacts>, String> {
    read_style_snapshot_phase(&serde_json::to_vec(wire).unwrap(), "root-style")
}

#[test]
fn style_snapshot_preserves_real_roots_shared_sources_and_build_references() {
    let (facts, wire) = encoded();
    assert_eq!(wire["schema"], STYLE_FACTS_SCHEMA);
    assert_eq!(wire["facts"][0]["bindings"], "not_requested");
    assert_eq!(wire["facts"][0]["body"], json!(facts[0].body));
    assert!(wire["facts"][0]["body"].get("children").is_none());
    assert_eq!(
        wire["facts"][0]["source"]["snapshot"],
        wire["facts"][1]["source"]["snapshot"]
    );
    assert_eq!(
        wire["facts"][1]["build"]["frontend_options"][KEY],
        json!({"kind":"census_reference","value":0})
    );
    let restored = read(&wire).unwrap();
    assert_eq!(json!(restored), json!(facts));
    let first = restored[0]
        .build
        .frontend_options
        .data::<Arc<Census>>()
        .unwrap();
    let second = restored[1]
        .build
        .frontend_options
        .data::<Arc<Census>>()
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(crate::SourceSnapshot::ptr_eq(
        restored[0].source.bytes.snapshot_range().unwrap().0,
        restored[1].source.bytes.snapshot_range().unwrap().0,
    ));
}

#[test]
fn style_binding_omission_is_distinct_from_missing_empty_or_fabricated_payloads() {
    let (_, wire) = encoded();
    for replacement in [json!([]), Value::Null, json!("unknown"), json!({})] {
        let mut changed = wire.clone();
        changed["facts"][0]["bindings"] = replacement;
        assert!(read(&changed).is_err());
    }
    let mut missing = wire.clone();
    missing["facts"][0]
        .as_object_mut()
        .unwrap()
        .remove("bindings");
    assert!(read(&missing).is_err());
    let mut missing = wire.clone();
    missing["facts"][0].as_object_mut().unwrap().remove("body");
    assert!(read(&missing).is_err());
    let mut fake_tree = wire;
    fake_tree["facts"][0]["body"]["children"] = json!([]);
    assert!(read(&fake_tree).is_err());
}

#[test]
fn style_snapshot_rejects_wrong_schema_phase_and_dangling_shared_references() {
    let (_, wire) = encoded();
    for schema in [RAW_FACTS_SCHEMA, "rstyle.diagnostic.style-facts/2", ""] {
        let mut changed = wire.clone();
        changed["schema"] = schema.into();
        assert!(read(&changed).is_err());
    }
    for phase in ["", "compiler-advisory-only"] {
        let mut changed = wire.clone();
        changed["phase"] = phase.into();
        assert!(read(&changed).is_err());
    }
    let bytes = serde_json::to_vec(&wire).unwrap();
    assert!(read_diagnostic_snapshot_phase(&bytes, "root-style").is_err());
    assert!(read_diagnostic_snapshot_headers_phase(&bytes, "root-style").is_err());
    let mut source = wire.clone();
    source["facts"][0]["source"]["snapshot"] = json!(999);
    assert!(read(&source).is_err());
    let mut census = wire;
    census["facts"][1]["build"]["frontend_options"][KEY] =
        json!({"kind":"census_reference","value":999});
    assert!(read(&census).is_err());
}
