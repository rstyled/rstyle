//! Independent bounded-source and structural-identity adversaries.
use super::{model, source};
use crate::concrete_types::Nominal;
use serde_json::{Value, json};

fn owner() -> Nominal {
    Nominal {
        source_identity: None,
        package_name: "sample".into(),
        package_version: "1".into(),
        manifest_digest: "unverified".into(),
        definition_path: "Adler32".into(),
        declaration_digest: "unverified".into(),
    }
}

fn argument() -> Value {
    json!([{
        "kind":"type", "value":"display is not authority", "display_only":true,
        "identity_status":"known", "identity":{
            "kind":"adt", "definition":{
                "path":"sample::Adler32", "definition_identity":"owner-identity",
                "crate":"sample", "stable_crate_id":"sample-build", "local":true
            }, "arguments":[]
        }
    }])
}

fn signature() -> Value {
    json!({"inputs":[], "output":argument(), "bound_variables":0,
        "abi":"\"Rust\"", "safety":"Safe", "variadic":false})
}

#[test]
fn concrete_default_identity_cannot_come_from_display_or_partial_arguments() {
    let baseline = argument();
    let decoded = serde_json::from_value::<Vec<model::Argument>>(baseline.clone()).unwrap();
    assert_eq!(
        model::nominal(&decoded, &owner()).unwrap().path,
        "sample::Adler32"
    );
    let mut attacks = Vec::new();
    for (key, value) in [
        ("kind", json!("const")),
        ("display_only", json!(false)),
        ("identity_status", json!("unknown")),
    ] {
        let mut attack = baseline.clone();
        attack[0][key] = value;
        attacks.push(attack);
    }
    for (key, value) in [
        ("path", json!("sample::Other")),
        ("definition_identity", json!("")),
        ("stable_crate_id", json!("")),
        ("local", json!(false)),
    ] {
        let mut attack = baseline.clone();
        attack[0]["identity"]["definition"][key] = value;
        attacks.push(attack);
    }
    let mut generic = baseline;
    generic[0]["identity"]["arguments"] = json!([{"kind":"type"}]);
    attacks.extend([generic, json!([])]);
    for attack in attacks {
        let decoded = serde_json::from_value::<Vec<model::Argument>>(attack).unwrap();
        assert!(model::nominal(&decoded, &owner()).is_err());
    }
}

#[test]
fn default_requires_safe_zero_argument_complete_concrete_signature() {
    let baseline = signature();
    let decoded: model::Signature = serde_json::from_value(baseline.clone()).unwrap();
    assert!(decoded.verify(&owner()).is_ok());
    for (key, value) in [
        ("inputs", json!([argument()])),
        ("output", json!([])),
        ("bound_variables", json!(1)),
        ("abi", json!("C")),
        ("abi", json!("Rust")),
        ("abi", json!("\"C\"")),
        ("safety", json!("Unsafe")),
        ("variadic", json!(true)),
    ] {
        let mut attack = baseline.clone();
        attack[key] = value;
        let decoded: model::Signature = serde_json::from_value(attack).unwrap();
        assert!(decoded.verify(&owner()).is_err());
    }
}

fn syntax(bytes: &str) -> Result<(), String> {
    let item: syn::ItemImpl = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let Some(syn::ImplItem::Fn(method)) = item.items.first() else {
        return Err("Missing original method".into());
    };
    source::syntax(&item, method).map(|_| ())
}

#[test]
fn original_default_body_is_not_restricted_to_a_constant_summary() {
    for body in [
        "Adler32 { a: 1, b: 0 }",
        "let a = 1; Adler32 { a, b: 0 }",
        "panic!(\"ordinary body still requires verification\")",
    ] {
        assert!(
            syntax(&format!(
                "impl Default for Adler32 {{ fn default() -> Self {{ {body} }} }}"
            ))
            .is_ok()
        );
    }
}

#[test]
fn default_source_rejects_receivers_generics_extra_members_and_other_signatures() {
    for bytes in [
        "impl Default for Adler32 { fn default(&self) -> Self { *self } }",
        "impl<T> Default for Adler32 { fn default() -> Self { todo!() } }",
        "impl Default for Adler32 { fn default<T>() -> Self { todo!() } }",
        "impl Default for Adler32 { fn default() -> u32 { 1 } }",
        "impl Default for Adler32 { unsafe fn default() -> Self { todo!() } }",
        "impl Default for Adler32 { async fn default() -> Self { todo!() } }",
        "impl Default for Adler32 { const fn default() -> Self { todo!() } }",
        "impl Default for Adler32 { fn default() -> Self { todo!() } const X:u32=1; }",
        "impl Adler32 { fn default() -> Self { todo!() } }",
    ] {
        assert!(syntax(bytes).is_err(), "{bytes}");
    }
}

fn call_inventory() -> Value {
    let requested = json!({
        "path":"core::default::Default::default", "definition_identity":"actual-core-default",
        "crate":"core", "stable_crate_id":"core-build", "local":false
    });
    let selected = json!({
        "path":"<Adler32 as core::default::Default>::default", "definition_identity":"selected-body",
        "crate":"sample", "stable_crate_id":"sample-build", "local":true
    });
    json!({"identity":"rstyle.rust.call-instance-observations/1", "complete":true,
    "calls":[{
        "hir":{"krate":0,"owner":7,"local":3},"span":{"start":5,"end":20},
        "expanded":false,"status":"resolved", "requested":requested,
        "selected":selected,"requested_arguments":argument(),"selected_arguments":[],
        "signature":{"display_only":true,"inputs":[],"output":"Adler32"}
    }]})
}

fn original_call(inventory: &Value) -> Result<(), String> {
    super::selection::original_call(
        inventory,
        &crate::HirRef {
            krate: 0,
            owner: 7,
            local: 3,
        },
        crate::ByteSpan { start: 5, end: 20 },
    )
    .map(|_| ())
}

#[test]
fn independent_original_inventory_requires_one_exact_complete_resolved_site() {
    let baseline = call_inventory();
    assert!(original_call(&baseline).is_ok());
    let mut incomplete = baseline.clone();
    incomplete["complete"] = json!(false);
    assert!(original_call(&incomplete).is_err());
    let mut wrong_identity = baseline.clone();
    wrong_identity["identity"] = json!("other");
    assert!(original_call(&wrong_identity).is_err());
    for (key, value) in [
        ("expanded", json!(true)),
        ("status", json!("unresolved")),
        ("span", json!({"start":4,"end":20})),
        ("hir", json!({"krate":0,"owner":7,"local":4})),
        ("compiled_dependency", json!({"identity":"foreign"})),
    ] {
        let mut attack = baseline.clone();
        attack["calls"][0][key] = value;
        assert!(original_call(&attack).is_err());
    }
    let mut duplicate = baseline;
    let repeated = duplicate["calls"][0].clone();
    duplicate["calls"].as_array_mut().unwrap().push(repeated);
    assert!(original_call(&duplicate).is_err());
}

#[test]
fn independent_default_selection_does_not_decode_unrelated_generic_argument_kinds() {
    let mut inventory = call_inventory();
    inventory["calls"].as_array_mut().unwrap().push(json!({
        "hir":{"krate":0,"owner":7,"local":8},"span":{"start":25,"end":40},
        "status":"unresolved", "requested_arguments":[{"kind":"const","value":"N"}]
    }));
    assert!(original_call(&inventory).is_ok());
}
