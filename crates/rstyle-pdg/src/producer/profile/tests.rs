//! Keep the former round trip only as an independent exact-output oracle.
use super::*;

const DESCRIPTION: &str = "R0 / compat.rust.hir-r0/0.1.0 / R0";

fn extension() -> Value {
    json!({"id":crate::R0_POLICY_ID,"version":crate::R0_POLICY_VERSION,
        "description":DESCRIPTION,"normal_flow_scope":DESCRIPTION,
        "literal":"R0", "unknown":[null,42,{"description":DESCRIPTION}],
        "dependency":{"id":crate::R0_POLICY_ID,"version":crate::R0_POLICY_VERSION,
            "description":DESCRIPTION,"nested":[{"normal_flow_scope":DESCRIPTION}]}})
}

fn seed(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if object.contains_key("description") {
                object.insert("description".into(), json!(DESCRIPTION));
                if object.contains_key("status") || object.contains_key("origin") {
                    object.insert("policy".into(), json!(crate::ProfileId::hir_r0()));
                }
            }
            for value in object.values_mut() {
                seed(value);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(seed),
        _ => {}
    }
}

fn pdg_fixture() -> crate::PdgDocument {
    let mut pdg: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor-compat/pdg.example.json"
    )))
    .unwrap();
    // Adapt the illustrative example's older conformance/edge shape locally.
    for edge in pdg["edges"].as_array_mut().unwrap() {
        edge["policy"] = json!(crate::ProfileId::hir_r0());
    }
    let conformance = pdg["extraction_evidence"]["conformance"].take();
    pdg["extraction_evidence"]["conformance"] = json!([{
        "profile":crate::ProfileId::hir_r0(), "status":conformance["status"],
        "reasons":conformance["reasons"]}]);
    pdg["function"]["reference_count"]["policy"] = json!(crate::ProfileId::hir_r0());
    seed(&mut pdg);
    let mut pdg: crate::PdgDocument = serde_json::from_value(pdg).unwrap();
    add_pdg_extensions(&mut pdg);
    pdg
}

fn add_pdg_extensions(pdg: &mut crate::PdgDocument) {
    pdg.function
        .inputs
        .iter_mut()
        .chain(&mut pdg.function.outputs)
        .for_each(|parameter| {
            parameter.extensions.insert("test:open".into(), extension());
        });
    for node in &mut pdg.nodes {
        node.attributes.description = Some(DESCRIPTION.into());
        node.attributes
            .extensions
            .insert("test:open".into(), extension());
        node.annotations.insert("test:open".into(), extension());
    }
    for edge in &mut pdg.edges {
        edge.annotations.insert("test:open".into(), extension());
    }
    pdg.memory_locations.push(crate::MemoryLocation {
        id: "m".into(),
        abstraction: "unknown".into(),
        description: DESCRIPTION.into(),
        evidence: pdg.nodes[0].evidence.clone(),
        extensions: std::collections::BTreeMap::from([("test:open".into(), extension())]),
    });
}

fn pair() -> Pair {
    let pdg = pdg_fixture();
    let cfg = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor-compat/cfg.example.json"
    )))
    .unwrap();
    let mut pair = Pair { pdg, cfg };
    for map in [
        &mut pair.pdg.extensions,
        &mut pair.cfg.extensions,
        &mut pair.pdg.producer.configuration,
        &mut pair.cfg.producer.configuration,
    ] {
        map.insert("test:open".into(), extension());
        map.insert("id".into(), json!(crate::R0_POLICY_ID));
        map.insert("version".into(), json!(crate::R0_POLICY_VERSION));
    }
    pair
}

#[test]
fn typed_policy_transport_matches_legacy_for_every_local_profile() {
    use ExtractionProfile::*;
    for profile in [
        ConstV1,
        CastsV1,
        MatchV1,
        GuardsV1,
        ConditionalV1,
        LoopsV1,
        CallsV1,
        HelpersV1,
        DataV1,
        DataV2,
        DataV3,
    ] {
        let pair = pair();
        let policy = profile.policy();
        let pdg: crate::PdgDocument = convert(&pair.pdg, &policy).unwrap();
        let cfg: crate::CfgDocument = convert(&pair.cfg, &policy).unwrap();
        let pair = transport::apply(pair, &policy).unwrap();
        assert_eq!(pair.pdg, pdg, "{profile:?}");
        assert_eq!(pair.cfg, cfg, "{profile:?}");
        assert_eq!(
            pair.pdg.extensions["test:open"]["dependency"],
            extension()["dependency"]
        );
    }
}

#[test]
fn unrelated_policies_and_non_description_source_text_remain_unchanged() {
    let mut pair = pair();
    pair.pdg.profile.version = "other-version".into();
    pair.cfg.sources[0].content = Some(DESCRIPTION.into());
    pair.pdg.nodes[0].attributes.literal = Some(crate::StringFact::known(DESCRIPTION));
    let policy = ExtractionProfile::HelpersV1.policy();
    let pdg: crate::PdgDocument = convert(&pair.pdg, &policy).unwrap();
    let cfg: crate::CfgDocument = convert(&pair.cfg, &policy).unwrap();
    let pair = transport::apply(pair, &policy).unwrap();
    assert_eq!(pair.pdg, pdg);
    assert_eq!(pair.cfg, cfg);
    assert_eq!(pair.pdg.profile.version, "other-version");
    assert_eq!(pair.cfg.sources[0].content.as_deref(), Some(DESCRIPTION));
}

#[test]
fn typed_transport_moves_graph_and_opaque_proof_buffers() {
    let mut pair = pair();
    pair.pdg.extensions.get_mut("test:open").unwrap()["dependency"]["large"] =
        json!("x".repeat(1024 * 1024));
    let id = pair.pdg.nodes[0].id.as_ptr();
    let proof = pair.pdg.extensions["test:open"]["dependency"]["large"]
        .as_str()
        .unwrap()
        .as_ptr();
    let pair = transport::apply(pair, &ExtractionProfile::HelpersV1.policy()).unwrap();
    assert_eq!(pair.pdg.nodes[0].id.as_ptr(), id);
    assert_eq!(
        pair.pdg.extensions["test:open"]["dependency"]["large"]
            .as_str()
            .unwrap()
            .as_ptr(),
        proof
    );
}

#[test]
fn malformed_typed_facts_keep_legacy_rejection_messages() {
    type Edit = fn(&mut Pair);
    let edits: [Edit; 6] = [
        |pair| pair.pdg.producer.language_version.value = Some("bad".into()),
        |pair| pair.cfg.producer.language_version.value = Some("bad".into()),
        |pair| {
            pair.pdg.nodes[0].attributes.literal = Some(crate::StringFact {
                status: crate::FactStatus::Known,
                value: Some("1".into()),
                reason: Some("bad".into()),
            })
        },
        |pair| pair.pdg.function.reference_count.policy = None,
        |pair| pair.pdg.function.outputs[0].variadic.reason = Some("bad".into()),
        |pair| pair.pdg.function.qualified_name.reason = Some(String::new()),
    ];
    let policy = ExtractionProfile::HelpersV1.policy();
    for edit in edits {
        let mut pair = pair();
        edit(&mut pair);
        let expected = convert::<crate::PdgDocument>(&pair.pdg, &policy)
            .and_then(|_| convert::<crate::CfgDocument>(&pair.cfg, &policy))
            .err()
            .unwrap();
        assert_eq!(transport::apply(pair, &policy).err().unwrap(), expected);
    }
}
