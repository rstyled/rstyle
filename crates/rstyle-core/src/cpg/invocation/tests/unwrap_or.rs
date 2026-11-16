//! The eager fallback can be discarded; the receiver's payload always moves into the result.

use super::{Invocation, definition, drop_type, node, nominal, reference, top, ty};
use crate::cpg::model::Document;
use crate::cpg::tree::{Body, Tables};
use crate::runtime_bounds::contracts::Contracts;
use serde_json::{Value, json};

const OPTION: &str = "unwrap:option";

fn document() -> Value {
    let mut document = super::document();
    let method = definition(&mut document);
    method["kind"] = json!("method");
    method["operation"] = json!("rust:Option.unwrap_or");
    method["callable"] = json!({
        "owner_type":OPTION,"receiver":OPTION,"parameters":["type:T"],
        "result":"type:T","generics":0
    });
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"unwrap:Option","kind":"type","name":"Option",
            "stable_identity":"core::option::Option","scope":"library","standard":true
        }));
    ty(
        &mut document,
        OPTION,
        "sum",
        json!({
            "key":"rust:Option","definition":"unwrap:Option","arguments":["type:u32"]
        }),
    );
    node(&mut document, "top.larger")["children"] = json!(["top.a", "top.b"]);
    node(&mut document, "top.a")["type"] = json!(OPTION);
    reference(&mut document, "top.larger")["dispatch"] = json!("method");
    reference(&mut document, "top.larger")["calls_back"] = json!({
        "value":true,"reason":"a generic parameter may contain workspace code"
    });
    payload(&mut document, "type:u32");
    document
}

fn payload(document: &mut Value, id: &str) {
    let types = document["unit"]["types"].as_array_mut().unwrap();
    let option = types.iter_mut().find(|ty| ty["id"] == OPTION).unwrap();
    option["arguments"] = json!([id]);
    node(document, "top.b")["type"] = json!(id);
    node(document, "top.larger")["type"] = json!(id);
    reference(document, "top.larger")["signature"] = json!({
        "parameters":[OPTION,id],"result":id
    });
}

fn certified(document: Value) -> bool {
    let document: Document = serde_json::from_value(document).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let function = unit
        .functions
        .as_ref()
        .unwrap()
        .iter()
        .find(|function| function.name == "top")
        .unwrap();
    let body = Body::new(function).unwrap();
    let tables = Tables::new(unit).unwrap();
    Invocation {
        body: &body,
        tables: &tables,
        call: body.node("top.larger").unwrap(),
        callee: tables.definition("def:larger").unwrap(),
        contracts: Contracts::embedded(),
    }
    .certified()
}

#[test]
fn unwrap_or_checks_the_fallback_without_requiring_discarded_receiver_payloads() {
    let document = document();
    assert!(certified(document.clone()));
    let option = document["unit"]["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|ty| ty["id"] == OPTION)
        .unwrap();
    assert!(option.get("variants").is_none());
    let role = Contracts::embedded()
        .callback_free("rust:Option.unwrap_or")
        .unwrap();
    assert_eq!(role.cleanup_free, [1]);
}

#[test]
fn fallback_references_to_owning_or_drop_types_are_cleanup_free() {
    for has_drop in [false, true] {
        let mut document = document();
        nominal(
            &mut document,
            "unwrap:owner",
            "record",
            json!({"fields":[]}),
        );
        if has_drop {
            document["unit"]["definitions"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap()["destructor"] = json!(true);
        }
        ty(
            &mut document,
            "unwrap:borrow",
            "reference",
            json!({
                "mode":"shared","target":"unwrap:owner"
            }),
        );
        payload(&mut document, "unwrap:borrow");
        assert!(certified(document), "owned referent drop={has_drop}");
    }
}

#[test]
fn an_owned_drop_fallback_remains_unresolved_even_though_the_receiver_value_moves() {
    let mut document = document();
    drop_type(&mut document);
    payload(&mut document, "test:drop");
    assert!(!certified(document));
}

#[test]
fn unknown_fallback_shapes_and_incomplete_borrow_targets_are_rejected() {
    let mut record = document();
    nominal(&mut record, "unwrap:incomplete", "record", json!({}));
    payload(&mut record, "unwrap:incomplete");
    assert!(!certified(record));
    for fields in [
        json!({"mode":"shared"}),
        json!({"mode":"shared","target":"unwrap:missing"}),
        json!({"mode":"owned","target":"type:u32"}),
    ] {
        let mut document = document();
        ty(&mut document, "unwrap:borrow", "reference", fields);
        payload(&mut document, "unwrap:borrow");
        assert!(!certified(document));
    }
    let mut opaque = document();
    ty(&mut opaque, "unwrap:unknown", "opaque", json!({}));
    payload(&mut opaque, "unwrap:unknown");
    assert!(!certified(opaque));
}

#[test]
fn named_execution_and_driven_implementations_are_never_refined_away() {
    for runs in [json!([]), json!([{"kind":"definition","id":"def:top"}])] {
        let mut document = document();
        reference(&mut document, "top.larger")["calls_back"] = json!({
            "value":true,"reason":"enumerated execution","runs":runs
        });
        assert!(!certified(document));
    }
    for drives in [json!([]), json!(["def:top"])] {
        let mut document = document();
        reference(&mut document, "top.larger")["drives"] = drives;
        assert!(!certified(document));
    }
    let mut no_reference = document();
    top(&mut no_reference)["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "top.larger");
    assert!(!certified(no_reference));
}
