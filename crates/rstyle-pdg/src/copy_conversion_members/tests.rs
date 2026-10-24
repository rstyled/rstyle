//! Real original compiler receipts; mutations do not generate the positive roster.
use super::*;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("original.json")).unwrap()
}

fn verify_fixture(value: &Value, key: &str) -> Result<(), String> {
    let observed: Observation = serde_json::from_value(value[key]["observation"].clone()).unwrap();
    let payload: TypeTree = serde_json::from_value(value["payload"].clone()).unwrap();
    let tagged: TypeTree = serde_json::from_value(value["tagged"].clone()).unwrap();
    let error: TypeTree = serde_json::from_value(value["error"].clone()).unwrap();
    let result = TypeTree::Result {
        ok: Box::new(payload.clone()),
        err: Box::new(error),
    };
    // Anchors come from the unchanged retained original, not the mutated receipt.
    let original = fixture();
    let anchors: Observation =
        serde_json::from_value(original[key]["observation"].clone()).unwrap();
    let owner = &original[key]["owner"];
    let compiled = &owner["compiled"];
    let owner = verify::Owner {
        name: owner["name"].as_str().unwrap(),
        identity: owner["identity"].as_str().unwrap(),
        crate_name: compiled["crate_name"].as_str().unwrap(),
        stable_crate_id: compiled["stable_crate_id"].as_str().unwrap(),
    };
    let contract = Contract {
        direction: if key == "wrap" {
            Direction::Wrap
        } else {
            Direction::Unwrap
        },
        input: if key == "wrap" { &payload } else { &tagged },
        output: if key == "wrap" { &tagged } else { &result },
        implementation_source: &anchors.implementation_source,
        method_source: &anchors.method_source,
        error_source: anchors.associated_error.as_ref().map(|error| &error.source),
    };
    let registry: crate::data_library::Registry =
        serde_json::from_str(include_str!("../../contracts/read-only-lengths-v2.json")).unwrap();
    let pins = authority::Pins {
        definitions: &registry.definitions,
    };
    verify::receipt(&observed, &contract, &owner, &pins)
}

#[test]
fn original_wrap_and_inverse_bind_actual_core_members_and_nominal_error() {
    for key in ["wrap", "unwrap"] {
        verify_fixture(&fixture(), key).unwrap();
    }
}

#[test]
fn selected_member_substitution_signature_and_source_tampering_reject() {
    for key in ["wrap", "unwrap"] {
        for pointer in [
            "/selected/definition_identity",
            "/method/definition_identity",
            "/trait_method/definition_identity",
            "/trait_definition/stable_crate_id",
            "/requested_signature/inputs/0/0/identity/definition/definition_identity",
            "/selected_signature/output/0/identity/definition/definition_identity",
            "/trait_arguments/1/identity/definition/path",
            "/requested_signature/abi",
            "/implementation_source/digest",
            "/method_source/digest",
        ] {
            let mut value = fixture();
            *value[key]["observation"].pointer_mut(pointer).unwrap() = "forged".into();
            assert!(verify_fixture(&value, key).is_err(), "{key}: {pointer}");
        }
    }
}

#[test]
fn selected_generic_arguments_missing_rosters_and_unsafe_headers_reject() {
    for key in ["wrap", "unwrap"] {
        for attack in ["arguments", "members", "unsafe", "authority", "direction"] {
            let mut value = fixture();
            let other = value[if key == "wrap" { "unwrap" } else { "wrap" }]["observation"]["trait_definition"].clone();
            let observed = &mut value[key]["observation"];
            match attack {
                "arguments" => observed["selected_arguments"] = observed["trait_arguments"].clone(),
                "members" => observed["members"].as_array_mut().unwrap().clear(),
                "unsafe" => observed["selected_signature"]["safety"] = "Unsafe".into(),
                "authority" => observed["admission_authority"] = true.into(),
                _ => observed["trait_definition"] = other,
            }
            assert!(verify_fixture(&value, key).is_err(), "{key}: {attack}");
        }
    }
}

#[test]
fn inverse_error_projection_member_and_payload_cannot_be_erased_or_changed() {
    for pointer in [
        "/associated_error/normalized_projection/0/identity/definition/definition_identity",
        "/associated_error/trait_member/definition_identity",
        "/associated_error/implementation_member/path",
        "/associated_error/source/digest",
        "/selected_signature/output/0/identity/arguments/1/definition/path",
    ] {
        let mut value = fixture();
        *value["unwrap"]["observation"].pointer_mut(pointer).unwrap() = "forged".into();
        assert!(verify_fixture(&value, "unwrap").is_err(), "{pointer}");
    }
    let mut value = fixture();
    value["unwrap"]["observation"]["associated_error"] = Value::Null;
    assert!(verify_fixture(&value, "unwrap").is_err());
}

#[test]
fn distinct_original_local_definitions_cannot_alias_compiler_identities() {
    for key in ["wrap", "unwrap"] {
        let mut value = fixture();
        let method = value[key]["observation"]["method"]["definition_identity"].clone();
        value[key]["observation"]["implementation"]["definition_identity"] = method;
        assert!(
            verify_fixture(&value, key).is_err(),
            "{key}: impl aliases method"
        );
    }
    let mut value = fixture();
    let method = value["unwrap"]["observation"]["method"]["definition_identity"].clone();
    value["unwrap"]["observation"]["associated_error"]["implementation_member"]["definition_identity"] =
        method.clone();
    value["unwrap"]["observation"]["members"][0]["definition"]["definition_identity"] = method;
    assert!(
        verify_fixture(&value, "unwrap").is_err(),
        "Error aliases method"
    );
}

#[test]
fn repeated_exact_endpoints_are_allowed_but_distinct_payload_paths_cannot_alias() {
    let mut value = fixture();
    verify_fixture(&value, "wrap").unwrap();
    let input = value["wrap"]["observation"]["trait_arguments"][1]["identity"]["definition"]["definition_identity"].clone();
    for pointer in [
        "/trait_arguments/0/identity/definition/definition_identity",
        "/requested_signature/output/0/identity/definition/definition_identity",
        "/selected_signature/output/0/identity/definition/definition_identity",
    ] {
        *value["wrap"]["observation"].pointer_mut(pointer).unwrap() = input.clone();
    }
    assert!(
        verify_fixture(&value, "wrap").is_err(),
        "tagged Self aliases payload input"
    );
}
