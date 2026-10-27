//! Structural adversaries supplement original-crate runtime and capture expectations.
use super::*;

fn registry() -> Registry {
    serde_json::from_str(include_str!("../../../contracts/read-only-lengths-v2.json")).unwrap()
}

fn argument(identity: Value) -> Value {
    json!({
        "kind":"type", "value":"display is not authority", "display_only":true,
        "identity_status":"known", "identity":identity,
    })
}

#[test]
fn never_result_requires_the_exact_uninhabited_structural_error() {
    let registry = registry();
    let result = argument(json!({
        "kind":"adt",
        "definition":{
            "path":"core::result::Result", "crate":"core", "local":false,
            "stable_crate_id":registry.definitions[authority::STABLE_CORE],
            "definition_identity":registry.definitions["std::result::Result"],
        },
        "arguments":[
            {"kind":"builtin", "name":"usize"}, {"kind":"builtin", "name":"!"}
        ],
    }));
    let original = json!([result]);
    output(&original, true, &registry).unwrap();

    for (pointer, changed) in [
        (
            "/0/identity/arguments/1",
            json!({"kind":"builtin", "name":"Infallible"}),
        ),
        (
            "/0/identity/arguments/1",
            json!({"kind":"builtin", "name":"unit"}),
        ),
        ("/0/identity/arguments/0/name", json!("u16")),
        (
            "/0/identity/definition/definition_identity",
            json!("another Result"),
        ),
        (
            "/0/identity/definition/stable_crate_id",
            json!("another core"),
        ),
        ("/0/identity_status", json!("unsupported")),
    ] {
        let mut forged = original.clone();
        *forged.pointer_mut(pointer).unwrap() = changed;
        assert!(output(&forged, true, &registry).is_err(), "{pointer}");
    }
    assert!(output(&json!([]), true, &registry).is_err());
    assert!(output(&json!([result.clone(), result]), true, &registry).is_err());
}

#[test]
fn concrete_obligation_arguments_preserve_order_arity_and_structural_identity() {
    let observed = json!([
        argument(json!({"kind":"builtin", "name":"usize"})),
        argument(json!({"kind":"builtin", "name":"u16"})),
    ]);
    arguments(&observed, &["usize", "u16"]).unwrap();
    assert!(arguments(&observed, &["u16", "usize"]).is_err());
    assert!(arguments(&observed, &["usize"]).is_err());

    for field in [
        "identity",
        "identity_status",
        "kind",
        "display_only",
        "value",
    ] {
        let mut forged = observed.clone();
        forged[0].as_object_mut().unwrap().remove(field);
        assert!(arguments(&forged, &["usize", "u16"]).is_err(), "{field}");
    }
    let unsupported = json!({"identity_status":"unsupported", "count":2});
    assert!(arguments(&unsupported, &["usize", "u16"]).is_err());
}

#[test]
fn display_agreement_cannot_replace_the_structural_result_proof() {
    let mut signature = json!({
        "display_only":true, "inputs":["u16"], "output":"std::result::Result<usize, !>"
    });
    display_signature(&signature, true).unwrap();
    for output in [
        "std::result::Result<usize, Infallible>",
        "usize",
        "std::result::Result<u16, !>",
    ] {
        signature["output"] = json!(output);
        assert!(display_signature(&signature, true).is_err());
    }
}
