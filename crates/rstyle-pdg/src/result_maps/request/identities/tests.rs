use super::*;
use serde_json::{Value, json};

fn definition(name: &str, identity: &str) -> Value {
    json!({"path": name, "definition_identity": identity,
        "crate": "sample", "stable_crate_id": "compiled-sample", "local": true})
}

fn adt(name: &str) -> Value {
    json!({"kind": "adt", "definition": definition(name, name), "arguments": []})
}

fn callback(input: &Value, output: &Value) -> Value {
    json!({"kind": "tuple_constructor", "definition": definition(
        output["definition"]["path"].as_str().unwrap(), "constructor"),
        "owner": output["definition"], "arguments": [], "signature": {
            "input": input, "output": output, "bound_variables": 0,
            "abi": "\"Rust\"", "safety": "Safe", "variadic": false}})
}

fn argument(value: &Value) -> Value {
    json!({"kind": "type", "value": "display", "display_only": true,
        "identity_status": "known", "identity": value})
}

fn call(types: [Value; 3], input: usize) -> Value {
    let arguments = [
        argument(&types[0]),
        argument(&types[1]),
        argument(&types[2]),
        argument(&callback(&types[input], &types[2])),
    ];
    json!({"hir": {"krate": 0, "owner": 1, "local": 1},
        "span": {"start": 0, "end": 1}, "expanded": true, "status": "resolved",
        "requested": definition("core::map", "method"), "selected": definition("core::map", "method"),
        "requested_arguments": arguments, "selected_arguments": arguments,
        "signature": {"display_only": true, "inputs": ["receiver", "callback"], "output": "result"}})
}

fn calls() -> [Value; 2] {
    [
        call([adt("Base"), adt("Error"), adt("Wrapper")], 0),
        call([adt("Wrapper"), adt("Error"), adt("WrappedError")], 1),
    ]
}

fn verify_raw(raw: [Value; 2]) -> Result<(), String> {
    let calls = raw.map(|call| serde_json::from_value(call).unwrap());
    check(
        &calls,
        &json!({"identity": "rstyle.rust.compiled-crate/1",
        "crate_name": "sample", "stable_crate_id": "compiled-sample"}),
    )
}

#[test]
fn exact_repeated_constructor_substitutions_are_accepted() {
    assert!(verify_raw(calls()).is_ok());
}

#[test]
fn stale_same_path_payload_and_constructor_identities_are_rejected() {
    for pointer in [
        "/selected_arguments/1/identity/definition/definition_identity",
        "/selected_arguments/0/identity/definition/stable_crate_id",
        "/selected_arguments/3/identity/owner/definition_identity",
        "/selected_arguments/3/identity/signature/input/definition/definition_identity",
        "/selected_arguments/3/identity/signature/output/definition/definition_identity",
        "/selected_arguments/3/identity/definition/path",
        "/selected_arguments/3/identity/definition/crate",
    ] {
        let mut raw = calls();
        *raw[1].pointer_mut(pointer).unwrap() = json!("unrelated");
        assert!(verify_raw(raw).is_err(), "{pointer}");
    }
}
