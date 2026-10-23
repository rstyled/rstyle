//! Actual captured substitutions plus independent role and ownership attacks.
use super::*;
use serde_json::{Value, json};

fn fixture() -> (Value, Structural, Structural) {
    let fixture: Value = serde_json::from_str(include_str!("fixture.json")).unwrap();
    let observed: Observation = serde_json::from_value(fixture["observation"].clone()).unwrap();
    let span = observed.result_type[0].identity.clone();
    let mut range = observed.receiver_type[0].identity.clone();
    let Structural::Adt { definition, .. } = &mut range else {
        panic!("actual Range must be an ADT");
    };
    definition.crate_hash = fixture["range"]["observation"]["definition"]["crate_hash"]
        .as_str()
        .map(str::to_owned);
    (fixture["observation"].clone(), span, range)
}

#[test]
fn original_range_selection_preserves_asymmetric_signatures_and_concrete_roles() {
    let (raw, span, range) = fixture();
    let observed: Observation = serde_json::from_value(raw.clone()).unwrap();
    verify(&observed, &span, &range).unwrap();
    assert!(observed.into.structure(&span, &[0, 1]).is_err());
    assert!(observed.from.structure(&span, &[0, 0]).is_err());
    let mut changed = raw;
    changed["receiver_type"][0]["value"] = json!("display is not authority");
    let observed = serde_json::from_value(changed).unwrap();
    verify(&observed, &span, &range).unwrap();
}

#[test]
fn wrong_substitution_scalar_hash_or_late_bound_signature_fails_closed() {
    let (raw, span, range) = fixture();
    for selection in ["into", "from"] {
        for field in ["requested_arguments", "substituted_impl_trait_arguments"] {
            let mut changed = raw.clone();
            changed[selection][field].as_array_mut().unwrap().reverse();
            let observed = serde_json::from_value(changed).unwrap();
            assert!(
                verify(&observed, &span, &range).is_err(),
                "{selection}/{field}"
            );
        }
        for signature in ["requested_signature", "selected_signature"] {
            let mut changed = raw.clone();
            changed[selection][signature]["bound_variables"] = json!(1);
            let observed = serde_json::from_value(changed).unwrap();
            assert!(verify(&observed, &span, &range).is_err());
        }
    }
    for (path, value) in [
        ("/receiver_type/0/identity/arguments/0/name", json!("u64")),
        (
            "/receiver_type/0/identity/definition/crate_hash",
            json!("changed"),
        ),
        (
            "/from/selected_arguments",
            raw["into"]["selected_arguments"].clone(),
        ),
        (
            "/from/original_impl_trait_arguments",
            raw["into"]["original_impl_trait_arguments"].clone(),
        ),
    ] {
        let mut changed = raw.clone();
        if path.ends_with("crate_hash") {
            changed["receiver_type"][0]["identity"]["definition"]["crate_hash"] = value;
        } else {
            *changed.pointer_mut(path).unwrap() = value;
        }
        let observed = serde_json::from_value(changed).unwrap();
        assert!(verify(&observed, &span, &range).is_err(), "{path}");
    }
}
