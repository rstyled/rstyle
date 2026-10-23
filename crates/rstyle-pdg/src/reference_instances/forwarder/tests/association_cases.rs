//! Fresh actual selections plus independently specified wrong-trait/call controls.
use super::*;

fn captures() -> Vec<Value> {
    let contexts: Vec<Value> =
        serde_json::from_str(include_str!("../association-fixtures.json")).unwrap();
    let mut result = Vec::new();
    for context in contexts {
        for instance in context["instances"].as_array().unwrap() {
            let mut case = instance.clone();
            case["context"] = context["context"].clone();
            case["forwarder"]["original"] = context["forwarder_original"].clone();
            case["constructor"]["original"] = context["constructor_original"].clone();
            result.push(case);
        }
    }
    result
}

fn check(forwarder: &Value, constructor: &Value) -> Result<()> {
    let record: Observation =
        serde_json::from_value(forwarder.clone()).map_err(|error| error.to_string())?;
    let target: Observation =
        serde_json::from_value(constructor.clone()).map_err(|error| error.to_string())?;
    let call = source::verify(&record.original)?;
    source::projection(&record, &target, call, &record.request.haystack)?;
    association::verify(&record.original, &record, &target, call)
}

#[test]
fn actual_default_and_no_default_slice_and_str_have_complete_outer_associations() {
    let cases = captures();
    assert_eq!(cases.len(), 4);
    for (case, (context, haystack)) in cases.iter().zip([
        ("default", "[u8]"),
        ("default", "str"),
        ("no-default", "[u8]"),
        ("no-default", "str"),
    ]) {
        assert_eq!(case["context"], context);
        assert_eq!(case["forwarder"]["request"]["haystack"], haystack);
        check(&case["forwarder"], &case["constructor"]).unwrap();
    }
    assert_ne!(
        cases[0]["forwarder"]["definition"],
        cases[2]["forwarder"]["definition"]
    );
}

#[test]
fn every_outer_definition_field_is_evidence_and_custom_from_cannot_substitute() {
    let case = &captures()[0];
    for definition in [
        "method",
        "implementation",
        "selected",
        "trait_definition",
        "trait_method",
    ] {
        for field in [
            "path",
            "definition_identity",
            "crate",
            "stable_crate_id",
            "local",
            "crate_hash",
        ] {
            if field == "crate_hash" && !definition.starts_with("trait_") {
                continue;
            }
            let mut changed = case["forwarder"].clone();
            let replacement = if field == "local" {
                let previous = case["forwarder"]["outer_trait_association"][definition][field]
                    .as_bool()
                    .unwrap();
                json!(!previous)
            } else {
                json!("custom::unreviewed")
            };
            changed["outer_trait_association"][definition][field] = replacement;
            assert!(
                check(&changed, &case["constructor"]).is_err(),
                "{definition}.{field}"
            );
        }
    }
}

#[test]
fn original_binders_concrete_payloads_and_selected_arguments_are_all_required() {
    let case = &captures()[0];
    for (path, replacement) in [
        (
            "/outer_trait_association/original_trait_arguments/1/value/lifetime/index",
            json!(1),
        ),
        (
            "/outer_trait_association/original_trait_arguments/1/value/referent/index",
            json!(0),
        ),
        (
            "/outer_trait_association/original_trait_arguments/0/value/arguments",
            json!([]),
        ),
        (
            "/outer_trait_association/substituted_trait_arguments/1/identity/mutable",
            json!(true),
        ),
        (
            "/outer_trait_association/substituted_trait_arguments/0/identity/arguments",
            json!([]),
        ),
        (
            "/outer_trait_association/selected_arguments/1/identity",
            json!({"kind":"str"}),
        ),
        (
            "/outer_trait_association/impl_arguments/0/identity",
            json!({"kind":"builtin","name":"u8"}),
        ),
        (
            "/outer_trait_association/substituted_trait_arguments/1/identity_status",
            json!("unsupported"),
        ),
    ] {
        let mut changed = case["forwarder"].clone();
        *changed.pointer_mut(path).unwrap() = replacement;
        assert!(check(&changed, &case["constructor"]).is_err(), "{path}");
    }
}

#[test]
fn incomplete_legacy_extra_evidence_and_cross_context_selections_fail_closed() {
    let cases = captures();
    let case = &cases[0];
    for (path, replacement) in [
        ("/outer_trait_association/identity", json!("unknown/1")),
        ("/outer_trait_association/complete", json!(false)),
        ("/outer_trait_association/selected_arguments", json!([])),
        (
            "/outer_trait_association/original_trait_arguments",
            json!([]),
        ),
        ("/outer_trait_association", Value::Null),
    ] {
        let mut changed = case["forwarder"].clone();
        *changed.pointer_mut(path).unwrap() = replacement;
        assert!(check(&changed, &case["constructor"]).is_err(), "{path}");
    }
    let mut changed = case["forwarder"].clone();
    changed["outer_trait_association"]["invented_purity"] = json!(true);
    assert!(check(&changed, &case["constructor"]).is_err());
    assert!(check(&case["forwarder"], &cases[2]["constructor"]).is_err());
    assert!(check(&case["forwarder"], &cases[1]["constructor"]).is_err());
}

#[test]
fn inner_call_cannot_select_another_helper_or_reorder_substitutions() {
    let case = &captures()[0];
    for path in [
        "/calls/0/hir/local",
        "/calls/0/requested/path",
        "/calls/0/selected/definition_identity",
        "/calls/0/requested_arguments/1/identity",
        "/calls/0/selected_arguments/1/identity",
    ] {
        let mut changed = case["forwarder"].clone();
        let replacement = if path.ends_with("/local") {
            json!(99999)
        } else if path.ends_with("/identity") {
            json!({"kind":"str"})
        } else {
            json!("other::constructor")
        };
        *changed.pointer_mut(path).unwrap() = replacement;
        assert!(check(&changed, &case["constructor"]).is_err(), "{path}");
    }
}

#[test]
fn native_association_authority_cannot_transfer_to_unreviewed_compiler_or_target() {
    let case = &captures()[0];
    for key in ["target_triple", "rustc_commit"] {
        let mut changed = case["forwarder"].clone();
        changed["original"]["build"][key]["value"] = json!("unreviewed");
        assert!(check(&changed, &case["constructor"]).is_err(), "{key}");
    }
}
