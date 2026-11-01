use super::*;

fn rejected(wire: &Value) {
    let bytes = serde_json::to_vec(wire).unwrap();
    assert!(read_diagnostic_snapshot_phase(&bytes, "compiler-advisory-only").is_err());
    assert!(read_diagnostic_snapshot_headers_phase(&bytes, "compiler-advisory-only").is_err());
}

#[test]
fn mixed_settings_preserve_exact_status_vectors_and_owner_observations() {
    let mut facts = roster();
    facts[1].build.cfgs.reverse();
    facts[1].build.cfgs.push("changed".into());
    facts[1].build.rustc_version =
        serde_json::from_value(json!({"status":"unknown","reason":"not captured"})).unwrap();
    let wire = encode_roster(&facts);
    assert_eq!(wire["build_settings"].as_array().unwrap().len(), 2);
    let bytes = serde_json::to_vec(&wire).unwrap();
    assert_eq!(read_diagnostic_snapshot(&bytes).unwrap(), facts);
    let headers = read_diagnostic_snapshot_headers_phase(&bytes, "compiler-advisory-only").unwrap();
    assert_eq!(headers[1].build, facts[1].build);
    assert_eq!(headers[1].source, facts[1].source);
}

#[test]
fn identical_settings_share_one_record_and_empty_rosters_need_an_empty_table() {
    let wire = encode_roster(&roster());
    assert_eq!(wire["build_settings"].as_array().unwrap().len(), 1);
    assert_eq!(wire["facts"][0]["build"]["settings"], 0);
    assert_eq!(wire["facts"][1]["build"]["settings"], 0);
    let empty = encode_roster(&[]);
    assert_eq!(empty["build_settings"], json!([]));
    assert!(
        read_diagnostic_snapshot(&serde_json::to_vec(&empty).unwrap())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn invalid_reference_types_ranges_and_contradictory_fields_fail() {
    let original = encode_roster(&roster());
    for reference in [
        json!(-1),
        json!(u64::MAX),
        json!("0"),
        Value::Null,
        json!(99),
    ] {
        let mut wire = original.clone();
        wire["facts"][0]["build"]["settings"] = reference;
        rejected(&wire);
    }
    let mut contradictory = original;
    contradictory["facts"][0]["build"]["cfgs"] = json!([]);
    rejected(&contradictory);
}

#[test]
fn missing_null_duplicate_unused_and_malformed_tables_fail() {
    let original = encode_roster(&roster());
    let entry = original["build_settings"][0].clone();
    for table in [
        Value::Null,
        json!({}),
        json!([]),
        json!([entry, entry]),
        json!([{}]),
    ] {
        let mut wire = original.clone();
        wire["build_settings"] = table;
        rejected(&wire);
    }
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("build_settings");
    rejected(&missing);
    let mut unused = original;
    unused["build_settings"].as_array_mut().unwrap().push(entry);
    unused["build_settings"][1]["cfgs"] = json!(["unused"]);
    rejected(&unused);
}

#[test]
fn settings_records_are_closed_and_cannot_supply_owner_options() {
    let original = encode_roster(&roster());
    for (key, value) in [("frontend_options", json!({})), ("admitted", json!(true))] {
        let mut wire = original.clone();
        wire["build_settings"][0][key] = value;
        rejected(&wire);
    }
}

#[test]
fn duplicate_reference_fields_are_rejected_by_both_readers() {
    let text = serde_json::to_string(&encode_roster(&roster())).unwrap();
    let duplicate = text.replacen("\"settings\":0", "\"settings\":0,\"settings\":0", 1);
    assert_ne!(duplicate, text);
    assert!(read_diagnostic_snapshot(duplicate.as_bytes()).is_err());
    assert!(
        read_diagnostic_snapshot_headers_phase(duplicate.as_bytes(), "compiler-advisory-only")
            .is_err()
    );
}

#[test]
fn obsolete_versions_and_inline_builds_are_rejected() {
    let facts = roster();
    let current = encode_roster(&facts);
    for schema in [
        "rstyle.diagnostic.raw-facts/2",
        "rstyle.diagnostic.raw-facts/3",
        "rstyle.diagnostic.raw-facts/4",
    ] {
        let mut wrong = current.clone();
        wrong["schema"] = schema.into();
        rejected(&wrong);
        wrong.as_object_mut().unwrap().remove("build_settings");
        rejected(&wrong);
    }
    let mut mixed = current;
    mixed["facts"][0]["build"] = serde_json::to_value(&facts[0].build).unwrap();
    rejected(&mixed);
}

#[test]
fn nonempty_bindings_and_original_body_identity_survive_shared_transport() {
    let mut facts = roster();
    let binding = crate::BindingFact {
        symbol: "binding".into(),
        name: "arg".into(),
        decl_span: facts[0].fn_span,
        scope: "scope:fixture".into(),
        hir: facts[0].body.hir.clone(),
        is_param: true,
        is_mut: false,
        ty: crate::ResolvedTy {
            normalized: Some("i32".into()),
            observed: "i32".into(),
            spelling: "i32".into(),
        },
    };
    facts[0].bindings.push(binding);
    let bytes = serde_json::to_vec(&encode_roster(&facts)).unwrap();
    assert_eq!(read_diagnostic_snapshot(&bytes).unwrap(), facts);
    let headers = read_diagnostic_snapshot_headers_phase(&bytes, "compiler-advisory-only").unwrap();
    assert_eq!(headers[0].bindings, facts[0].bindings);
    assert_eq!(headers[0].body, facts[0].body.hir);
}

#[test]
fn literal_metadata_is_not_interpreted_as_a_transport_reference() {
    let mut facts = roster();
    for fact in &mut facts {
        fact.build.frontend_options.insert(
            "invocation".into(),
            json!({"kind":"shared_reference","value":999}),
        );
    }
    let wire = encode_roster(&facts);
    assert_eq!(
        read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).unwrap(),
        facts
    );
}
