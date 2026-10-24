use super::*;

fn roster() -> Vec<HirFnFacts> {
    let mut first: HirFnFacts = serde_json::from_str(include_str!(
        "../../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let context = crate::nominal_boundary::tests::context();
    let source: SourceSnapshot = "// shared complete selected unit\nfn f(){}".into();
    let census = std::sync::Arc::new(crate::nominal_boundary::census::Census {
        identity: crate::nominal_boundary::census::IDENTITY.into(),
        files: vec![crate::nominal_boundary::census::UnitFile {
            path: context.root_source.clone(),
            digest: source.identity().into(),
            bytes: source,
        }],
        context,
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    });
    first.build.frontend_options.insert_data(census);
    let mut second = first.clone();
    second.name = "other".into();
    vec![first, second]
}

fn encoded() -> (Vec<HirFnFacts>, serde_json::Value) {
    let facts = roster();
    let mut bytes = Vec::new();
    write_diagnostic_snapshot(&mut bytes, "before-helper-certification", &facts).unwrap();
    (facts, serde_json::from_slice(&bytes).unwrap())
}

#[test]
fn full_roster_and_evidence_roundtrip_with_shared_sources() {
    let (original, wire) = encoded();
    let decoded = read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(decoded).unwrap()
    );
    assert_eq!(wire["facts"].as_array().unwrap().len(), 2);
    let sources = wire["sources"].as_array().unwrap();
    assert_eq!(
        sources
            .iter()
            .filter(|source| source.as_str() == Some("fn f(){}"))
            .count(),
        1
    );
    assert_eq!(
        wire["facts"][0]["source"]["snapshot"],
        wire["facts"][1]["source"]["snapshot"]
    );
    assert_eq!(
        sources
            .iter()
            .filter(|source| source.as_str() == Some("// shared complete selected unit\nfn f(){}"))
            .count(),
        1
    );
    assert!(wire.get("graphs").is_none());
    assert!(wire.get("certified").is_none());
}

#[test]
fn malformed_source_references_and_unused_inputs_fail() {
    let (_, original) = encoded();
    let mut wire = original.clone();
    wire["facts"][0]["source"]["snapshot"] = 999999.into();
    assert!(read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).is_err());
    let mut wire = original.clone();
    wire["facts"][0]["source"]["range"]["end"] = 999999.into();
    assert!(read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).is_err());
    let mut wire = original.clone();
    wire["sources"]
        .as_array_mut()
        .unwrap()
        .push("orphan".into());
    assert!(read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).is_err());
    let mut wire = original;
    wire["schema"] = "rstyle.shared-unit/5".into();
    assert!(read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).is_err());
}

#[test]
fn diagnostic_input_cannot_carry_graph_or_certification_authority() {
    let (_, original) = encoded();
    for key in ["graphs", "certified", "helpers", "admitted"] {
        let mut wire = original.clone();
        wire[key] = serde_json::json!([]);
        assert!(read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).is_err());
    }
}
