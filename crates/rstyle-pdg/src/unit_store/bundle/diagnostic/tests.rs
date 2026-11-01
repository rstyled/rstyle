use super::*;
use crate::nominal_boundary::census::{Census, KEY};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn roster() -> Vec<HirFnFacts> {
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
        serde_json::to_value(&original).unwrap(),
        serde_json::to_value(&decoded).unwrap()
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

fn census_entry(function: &Value) -> &Value {
    &function["build"]["frontend_options"][KEY]
}

fn expanded_censuses(wire: &Value) -> Value {
    let mut expanded = wire.clone();
    let mut entries = Vec::new();
    for function in expanded["facts"].as_array_mut().unwrap() {
        let entry = &mut function["build"]["frontend_options"][KEY];
        match entry["kind"].as_str() {
            Some("census") => entries.push(entry.clone()),
            Some("census_reference") => {
                *entry = entries[entry["value"].as_u64().unwrap() as usize].clone();
            }
            _ => {}
        }
    }
    expanded
}

fn encode_roster(facts: &[HirFnFacts]) -> Value {
    let mut bytes = Vec::new();
    write_diagnostic_snapshot(&mut bytes, "compiler-advisory-only", facts).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn repeated_owners_share_census_metadata_and_decoded_immutable_ownership() {
    let first = roster().remove(0);
    let facts: Vec<_> = (0..32)
        .map(|index| {
            let mut function = first.clone();
            function.name = format!("owner_{index}");
            function
        })
        .collect();
    let wire = encode_roster(&facts);
    assert_eq!(wire["schema"], RAW_FACTS_SCHEMA);
    let functions = wire["facts"].as_array().unwrap();
    assert_eq!(census_entry(&functions[0])["kind"], "census");
    for function in &functions[1..] {
        assert_eq!(
            census_entry(function),
            &json!({"kind":"census_reference","value":0})
        );
    }
    let expanded = expanded_censuses(&wire);
    let inline_bytes = serde_json::to_vec(census_entry(&functions[0]))
        .unwrap()
        .len();
    let reference_bytes = serde_json::to_vec(census_entry(&functions[1]))
        .unwrap()
        .len();
    assert_eq!(
        serde_json::to_vec(&expanded).unwrap().len() - serde_json::to_vec(&wire).unwrap().len(),
        31 * (inline_bytes - reference_bytes)
    );
    let decoded = read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).unwrap();
    let census = decoded[0]
        .build
        .frontend_options
        .data::<Arc<Census>>()
        .unwrap();
    for (original, function) in facts.iter().zip(&decoded) {
        assert_eq!(
            serde_json::to_value(original).unwrap(),
            serde_json::to_value(function).unwrap()
        );
        let shared = function
            .build
            .frontend_options
            .data::<Arc<Census>>()
            .unwrap();
        assert!(Arc::ptr_eq(&census, &shared));
    }
}

#[test]
fn equal_independent_census_owners_and_different_contexts_keep_distinct_ids() {
    let mut facts = roster();
    let original = facts[0]
        .build
        .frontend_options
        .data::<Arc<Census>>()
        .unwrap();
    let equal = Arc::new((**original).clone());
    let mut other = (**original).clone();
    other.context.crate_name = "different_context".into();
    facts[1]
        .build
        .frontend_options
        .insert_data(Arc::clone(&equal));
    let mut third = facts[0].clone();
    third.build.frontend_options.insert_data(Arc::new(other));
    let mut fourth = facts[1].clone();
    fourth.name = "equal_owner_again".into();
    facts.extend([third, fourth]);
    let wire = encode_roster(&facts);
    for index in 0..3 {
        assert_eq!(census_entry(&wire["facts"][index])["kind"], "census");
    }
    assert_eq!(
        census_entry(&wire["facts"][3]),
        &json!({"kind":"census_reference","value":1})
    );
    let decoded = read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).unwrap();
    let census: Vec<_> = decoded
        .iter()
        .map(|function| {
            function
                .build
                .frontend_options
                .data::<Arc<Census>>()
                .unwrap()
                .into_owned()
        })
        .collect();
    assert_eq!(census[0], census[1]);
    assert!(!Arc::ptr_eq(&census[0], &census[1]));
    assert_ne!(census[0].context, census[2].context);
    assert!(Arc::ptr_eq(&census[1], &census[3]));
}

#[test]
fn census_references_require_prior_owner_exact_key_and_current_schema() {
    let (_, wire) = encoded();
    for (index, reference) in [
        (0, json!({"kind":"census_reference","value":0})),
        (1, json!({"kind":"census_reference","value":1})),
        (1, json!({"kind":"census_reference","value":u32::MAX})),
        (1, json!({"kind":"census_reference","value":-1})),
        (1, json!({"kind":"census_reference","value":4294967296u64})),
        (1, json!({"kind":"census_reference","value":null})),
        (1, json!({"kind":"census_reference","value":"0"})),
        (
            1,
            json!({"kind":"census_reference","value":0,"trusted":true}),
        ),
    ] {
        let mut changed = wire.clone();
        changed["facts"][index]["build"]["frontend_options"][KEY] = reference;
        assert!(read_diagnostic_snapshot(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    let mut wrong = wire.clone();
    wrong["facts"][1]["build"]["frontend_options"][crate::declaration_origins::KEY] =
        json!({"kind":"census_reference","value":0});
    assert!(read_diagnostic_snapshot(&serde_json::to_vec(&wrong).unwrap()).is_err());
    for schema in [
        "rstyle.diagnostic.raw-facts/3",
        "rstyle.diagnostic.raw-facts/2",
    ] {
        let mut legacy = wire.clone();
        legacy["schema"] = schema.into();
        assert!(read_diagnostic_snapshot(&serde_json::to_vec(&legacy).unwrap()).is_err());
    }
}

fn mixed_census_roster() -> Vec<HirFnFacts> {
    let valid = roster().remove(0);
    let mut absent = valid.clone();
    absent.name = "absent_census".into();
    absent.build.frontend_options.remove(KEY);
    absent
        .build
        .frontend_options
        .insert("unrelated_null".into(), Value::Null);
    let mut malformed = valid.clone();
    malformed.name = "malformed_census".into();
    malformed
        .build
        .frontend_options
        .insert(KEY.into(), json!({"files":"unavailable"}));
    let mut repeated = valid.clone();
    repeated.name = "repeated_valid_census".into();
    vec![absent, malformed, valid, repeated]
}

#[test]
fn absent_and_malformed_census_observations_do_not_consume_owner_ids() {
    let facts = mixed_census_roster();
    let wire = encode_roster(&facts);
    assert!(
        wire["facts"][0]["build"]["frontend_options"]
            .get(KEY)
            .is_none()
    );
    assert_eq!(
        census_entry(&wire["facts"][1]),
        &json!({
            "kind":"inline", "value":{"files":"unavailable"}
        })
    );
    assert_eq!(census_entry(&wire["facts"][2])["kind"], "census");
    assert_eq!(
        census_entry(&wire["facts"][3]),
        &json!({"kind":"census_reference","value":0})
    );
    let decoded = read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&facts).unwrap(),
        serde_json::to_value(&decoded).unwrap()
    );
    mixed_census_ownership(&decoded);
}

fn mixed_census_ownership(decoded: &[HirFnFacts]) {
    assert_eq!(
        decoded[0].build.frontend_options.get("unrelated_null"),
        Some(&Value::Null)
    );
    assert!(
        decoded[0]
            .build
            .frontend_options
            .data::<Arc<Census>>()
            .is_err()
    );
    assert!(
        decoded[1]
            .build
            .frontend_options
            .data::<Arc<Census>>()
            .is_err()
    );
    let first = decoded[2]
        .build
        .frontend_options
        .data::<Arc<Census>>()
        .unwrap();
    let second = decoded[3]
        .build
        .frontend_options
        .data::<Arc<Census>>()
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
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

#[test]
fn sealed_advisory_publication_requires_exact_phase_and_grants_no_strict_import() {
    let facts = roster();
    let mut bytes = Vec::new();
    let manifest =
        write_sealed_diagnostic_snapshot(&mut bytes, "compiler-advisory-only", &facts).unwrap();
    let snapshot = manifest.verify_owned(bytes).unwrap();
    let decoded =
        read_diagnostic_snapshot_phase(snapshot.bytes(), "compiler-advisory-only").unwrap();
    assert_eq!(decoded.len(), facts.len());
    assert!(read_diagnostic_snapshot_phase(snapshot.bytes(), "pre-helper-certification").is_err());
    assert!(crate::unit_store::UnitStore::from_bundle(snapshot.bytes()).is_err());
    assert!(
        crate::unit_store::UnitStore::from_verified_current_run(&snapshot, |_| Ok(())).is_err()
    );

    let (_, diagnostic) = encoded();
    assert!(
        read_diagnostic_snapshot_phase(
            &serde_json::to_vec(&diagnostic).unwrap(),
            "compiler-advisory-only"
        )
        .is_err()
    );
}

struct FlushFailure;

impl std::io::Write for FlushFailure {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("injected advisory flush failure"))
    }
}

#[test]
fn failed_raw_publication_flush_cannot_return_a_seal() {
    assert!(
        write_sealed_diagnostic_snapshot(FlushFailure, "compiler-advisory-only", &[])
            .unwrap_err()
            .contains("advisory flush failure")
    );
}

#[test]
fn header_reader_preserves_every_nonbody_field_and_shared_source() {
    let (original, wire) = encoded();
    let headers = read_diagnostic_snapshot_headers_phase(
        &serde_json::to_vec(&wire).unwrap(),
        "before-helper-certification",
    )
    .unwrap();
    let mut expected = serde_json::to_value(&original).unwrap();
    for (value, facts) in expected.as_array_mut().unwrap().iter_mut().zip(&original) {
        value["body"] = serde_json::to_value(&facts.body.hir).unwrap();
    }
    assert_eq!(serde_json::to_value(&headers).unwrap(), expected);
    assert!(crate::SourceSnapshot::ptr_eq(
        headers[0].source.bytes.snapshot_range().unwrap().0,
        headers[1].source.bytes.snapshot_range().unwrap().0,
    ));
}

#[test]
fn header_reader_preserves_snapshot_reference_and_phase_validation() {
    let (_, wire) = encoded();
    let mut bad_schema = wire.clone();
    bad_schema["schema"] = json!("unknown");
    let mut bad_source = wire.clone();
    bad_source["facts"][0]["source"]["snapshot"] = json!(u32::MAX);
    let mut unused_source = wire.clone();
    unused_source["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!("unused"));
    let mut bad_inventory = wire.clone();
    bad_inventory["inherent_units"] = json!([{}]);
    let mut extra_field = wire.clone();
    extra_field["certified"] = json!(true);
    for malformed in [
        bad_schema,
        bad_source,
        unused_source,
        bad_inventory,
        extra_field,
    ] {
        let bytes = serde_json::to_vec(&malformed).unwrap();
        assert!(read_diagnostic_snapshot(&bytes).is_err());
        assert!(
            read_diagnostic_snapshot_headers_phase(&bytes, "before-helper-certification").is_err()
        );
    }
    assert!(
        read_diagnostic_snapshot_headers_phase(&serde_json::to_vec(&wire).unwrap(), "other-phase",)
            .is_err()
    );
}

mod settings;
