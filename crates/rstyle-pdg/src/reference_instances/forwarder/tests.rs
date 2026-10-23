//! Actual retained forwarders; their direct-call contract has independent negatives.
mod association_cases;
use super::*;
use serde_json::{Value, json};

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

#[test]
fn retained_slice_and_str_forwarders_have_exact_direct_argument_flow() {
    for raw in fixtures() {
        let record: Observation = serde_json::from_value(raw).unwrap();
        assert_eq!(record.original.bindings[0].ty.spelling, "");
        assert_eq!(record.original.signature.params[0].ty.spelling, "&'h H");
        let call = source::verify(&record.original).unwrap();
        source::projection(&record, &record, call, &record.request.haystack).unwrap();
        // Legacy evidence lacks outer trait selection, so body proof is unavailable.
        assert!(
            association::verify(&record.original, &record, &record, call)
                .unwrap_err()
                .contains("Legacy")
        );
    }
}

#[test]
fn original_type_roles_require_source_spelling_only_on_the_signature() {
    let raw = &fixtures()[0];
    for role in ["signature", "binding", "pattern", "read", "normalized"] {
        let mut facts: HirFnFacts = serde_json::from_value(raw["original"].clone()).unwrap();
        match role {
            "signature" => facts.signature.params[0].ty.spelling.clear(),
            "binding" => facts.bindings[0].ty.spelling = "&'h H".into(),
            "pattern" => {
                facts.signature.params[0].pat.ty.as_mut().unwrap().spelling = "&'h H".into()
            }
            "read" => {
                facts.body.children[0].children[0]
                    .ty
                    .as_mut()
                    .unwrap()
                    .observed = "&Other".into()
            }
            "normalized" => facts.signature.params[0].ty.normalized = Some("u8".into()),
            _ => unreachable!(),
        }
        assert!(source::verify(&facts).is_err(), "{role}");
    }
}

#[test]
fn source_contract_rejects_additional_effects_wrong_origins_and_payload_changes() {
    let raw = &fixtures()[0];
    let facts: HirFnFacts = serde_json::from_value(raw["original"].clone()).unwrap();
    for text in [
        "fn from(haystack: &'h H) -> Input<'h> { effect(); Input::new(haystack) }",
        "fn from(haystack: &'h H) -> Input<'h> { Input::new(other) }",
        "fn from(haystack: &'h H) -> Input<'h> { Other::new(haystack) }",
        "fn from(haystack: &'h H) -> Input<'h> { Input::new(&make()) }",
        "fn from(haystack: &'h H) -> Input<'h> { Input::new(haystack, haystack) }",
        "fn from(haystack: &'h H) -> Input<'h> { return Input::new(haystack); }",
    ] {
        let mut changed = facts.clone();
        changed.source.bytes = text.into();
        assert!(source::verify(&changed).is_err(), "{text}");
    }
}

#[test]
fn complete_hir_and_binding_censuses_reject_omissions_effects_and_wrong_formal() {
    let raw = &fixtures()[0];
    for path in ["binding", "read", "call", "block", "formal"] {
        let mut facts: HirFnFacts = serde_json::from_value(raw["original"].clone()).unwrap();
        match path {
            "binding" => facts.bindings.clear(),
            "read" => {
                facts.body.children[0].children[0]
                    .res
                    .as_mut()
                    .unwrap()
                    .target
                    .as_mut()
                    .unwrap()
                    .local += 1
            }
            "call" => facts.body.children[0].is_unsafe_block = true,
            "block" => facts.body.children.push(facts.body.children[0].clone()),
            "formal" => facts.signature.params[0].pat.binding_mut = Some(true),
            _ => unreachable!(),
        }
        assert!(source::verify(&facts).is_err(), "{path}");
    }
}

#[test]
fn projected_roster_requires_every_exact_node_type_and_complete_result() {
    let raw = &fixtures()[0];
    let target: Observation = serde_json::from_value(raw.clone()).unwrap();
    for mode in ["omitted", "duplicate", "reordered", "result", "formal"] {
        let mut changed = raw.clone();
        match mode {
            "omitted" => {
                changed["nodes"].as_array_mut().unwrap().pop();
            }
            "duplicate" => changed["nodes"][1] = changed["nodes"][0].clone(),
            "reordered" => changed["nodes"].as_array_mut().unwrap().swap(0, 1),
            "result" => changed["projected_result"][0]["identity"]["arguments"] = json!([]),
            "formal" => changed["nodes"][0]["ty"][0]["identity"]["mutable"] = json!(true),
            _ => unreachable!(),
        }
        let record: Observation = serde_json::from_value(changed).unwrap();
        let call = source::verify(&record.original).unwrap();
        assert!(
            source::projection(&record, &target, call, "[u8]").is_err(),
            "{mode}"
        );
    }
}
