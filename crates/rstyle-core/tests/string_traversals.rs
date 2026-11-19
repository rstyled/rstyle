//! Standard string sources reuse the shared traversal model with a byte-length upper bound.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::{self, Assessment, Bound, Outcome, Program, Root};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const PRODUCTION: &str = include_str!("../src/cpg/testdata/string-traversal-production.cpg.json");
const PROVENANCE: &str =
    include_str!("../src/cpg/testdata/string-traversal-production.provenance.json");

fn function<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == name)
        .unwrap()
}

fn node<'a>(function: &'a mut Value, id: &str) -> &'a mut Value {
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn source_definition(document: &mut Value) -> &mut Value {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "string:source")
        .unwrap()
}

fn program(document: Value, name: &str) -> Program {
    let programs = rstyle_core::cpg::run_bounds(vec![document]).unwrap();
    let (_, input) = programs
        .into_iter()
        .find(|(identity, _)| identity.ends_with(&format!("::{name}")))
        .unwrap();
    let Input::Captured { program } = input else {
        panic!("expected captured input: {input:?}");
    };
    program
}

fn proven(program: &Program) -> Assessment {
    let result = runtime_bounds::analyze(program);
    let Outcome::Proven(assessment) = result else {
        panic!("expected a proven string traversal: {result:?}");
    };
    assert!(!assessment.loops.is_empty());
    assert!(assessment.loop_body_entries.is_complete());
    assessment
}

fn unknown(document: Value) {
    let result = runtime_bounds::analyze(&program(document, "count"));
    assert!(matches!(result, Outcome::Unknown { .. }), "{result:?}");
}

fn add_string_types(document: &mut Value) {
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"string:str", "kind":"string", "key":"rust:str", "spelling":"str"}),
        json!({"id":"string:shared", "kind":"reference", "mode":"shared",
               "target":"string:str", "spelling":"&str"}),
        json!({"id":"string:char", "kind":"char", "key":"rust:char", "spelling":"char"}),
        json!({"id":"string:byte", "kind":"integer", "key":"rust:u8", "spelling":"u8",
               "bits":8, "signed":false}),
        json!({"id":"string:usize", "kind":"integer", "key":"rust:usize", "spelling":"usize",
               "bits":64, "signed":false, "pointer_sized":true}),
        json!({"id":"string:indexed", "kind":"tuple", "spelling":"(usize, char)",
               "items":["string:usize","string:char"]}),
    ]);
}

fn add_source(document: &mut Value, operation: &str) {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"string:source", "kind":"method", "name":operation,
            "stable_identity":format!("std::{operation}"), "scope":"library", "standard":true,
            "operation":format!("rust:str.{operation}"),
            "callable":{"owner_type":"string:str", "receiver":"string:shared", "parameters":[],
                        "result":"type:iter", "generics":0, "modifiers":[]}
        }));
    let function = function(document, "count");
    let span = node(function, "count.items")["span"].clone();
    function["nodes"].as_array_mut().unwrap().push(json!({
        "id":"string:source-call", "kind":"call", "original_kind":"expr_method_call",
        "children":["count.items"], "type":"type:iter", "span":span,
        "provenance":{"origin":"source"}
    }));
    node(function, "count.into_iter")["children"] = json!(["string:source-call"]);
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"string:source-call", "status":"resolved", "dispatch":"method",
        "target":{"kind":"definition","id":"string:source"},
        "calls_back":{"value":false,"reason":"The exact inherent str source has no callable argument"}
    }));
}

fn empty_scan(function: &mut Value, item: &str) {
    function["signature"]["parameters"][0]["type"] = json!("string:shared");
    function["signature"]["result"] = json!("type:unit");
    node(function, "count.block")["children"] = json!(["count.stmt_for"]);
    node(function, "count.block")["type"] = json!("type:unit");
    node(function, "count.loop_body")["children"] = json!([]);
    node(function, "count.item_pat")["type"] = json!(item);
    let receiver = node(function, "count.items");
    receiver["type"] = json!("string:shared");
    receiver["adjustments"] = json!([
        {"kind":"auto_deref", "target":"string:str"},
        {"kind":"auto_ref", "mode":"shared", "target":"string:shared"}
    ]);
    for symbol in function["symbols"].as_array_mut().unwrap() {
        if symbol["id"] == "count.items" {
            symbol["type"] = json!("string:shared");
        } else if symbol["id"] == "count.item" {
            symbol["type"] = json!(item);
        }
    }
    for place in function["places"].as_array_mut().unwrap() {
        if place["node"] == "count.items" {
            place["type"] = json!("string:shared");
        }
    }
}

fn prune_scan(function: &mut Value) {
    let mut retained = vec!["count.block".to_owned()];
    let mut index = 0;
    while index < retained.len() {
        let children = node(function, &retained[index])["children"].clone();
        for child in children.as_array().unwrap() {
            let child = child.as_str().unwrap().to_owned();
            if !retained.contains(&child) {
                retained.push(child);
            }
        }
        index += 1;
    }
    for table in ["nodes", "refs", "accesses", "places", "values"] {
        function[table].as_array_mut().unwrap().retain(|entry| {
            let id = if table == "nodes" {
                &entry["id"]
            } else {
                &entry["node"]
            };
            id.as_str()
                .is_some_and(|id| retained.iter().any(|kept| kept == id))
        });
    }
    function["symbols"]
        .as_array_mut()
        .unwrap()
        .retain(|symbol| {
            symbol["kind"] == "parameter"
                || symbol["node"]
                    .as_str()
                    .is_some_and(|id| retained.iter().any(|kept| kept == id))
        });
    function["cfg"] =
        json!({"entry":"count.block", "exits":[{"id":"count.exit","kind":"normal"}], "edges":[]});
}

fn traversal(operation: &str) -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    add_string_types(&mut document);
    let item = match operation {
        "bytes" => "string:byte",
        "char_indices" => "string:indexed",
        "lines" => "string:shared",
        _ => "string:char",
    };
    empty_scan(function(&mut document, "count"), item);
    add_source(&mut document, operation);
    prune_scan(function(&mut document, "count"));
    document
}

#[test]
fn utf8_byte_lengths_bound_every_admitted_standard_string_source() {
    for operation in ["chars", "bytes", "char_indices", "lines"] {
        let program = program(traversal(operation), "count");
        let assessment = proven(&program);
        assert_eq!(program.extents.len(), 1);
        let extent = &program.extents[0];
        assert_eq!(extent.root, Root::Formal(0));
        assert!(extent.fields.is_empty());
        assert_eq!(
            assessment.loop_body_entries,
            Bound::Input(extent.input.clone())
        );
        for text in ["", "ascii", "é水🦀", "\n", "a\r\n水\n", "\n\n"] {
            let actual = match operation {
                "chars" => text.chars().count(),
                "bytes" => text.bytes().fold(0, |count, _| count + 1),
                "char_indices" => text.char_indices().count(),
                "lines" => text.lines().count(),
                _ => unreachable!(),
            };
            let bound = assessment
                .loop_body_entries
                .evaluate(&|_| Some(text.len() as u64));
            assert_eq!(bound, Some(text.len() as u64));
            assert!(actual as u64 <= bound.unwrap(), "{operation}: {text:?}");
        }
    }
}

#[test]
fn unsupported_string_operations_do_not_inherit_a_byte_length_contract() {
    for operation in [
        "split",
        "rsplit",
        "split_whitespace",
        "encode_utf16",
        "custom",
    ] {
        unknown(traversal(operation));
    }
}

#[test]
fn matching_spelling_does_not_certify_custom_or_trait_dispatch() {
    let mut custom = traversal("chars");
    source_definition(&mut custom)["standard"] = json!(false);
    unknown(custom);

    let mut trait_method = traversal("chars");
    source_definition(&mut trait_method)["implements"] = json!("def:Iterator.next");
    unknown(trait_method);

    let mut unresolved = traversal("chars");
    let reference = function(&mut unresolved, "count")["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "string:source-call")
        .unwrap();
    reference["status"] = json!("unresolved");
    reference.as_object_mut().unwrap().remove("target");
    unknown(unresolved);
}

fn add_diverging_string_function(document: &mut Value) {
    let mut spin = function(document, "count").clone();
    let span = spin["span"].clone();
    spin["id"] = json!("string:spin-function");
    spin["definition"] = json!("string:spin-definition");
    spin["name"] = json!("spin_text");
    spin["root"] = json!("string:spin-loop");
    spin["signature"]["parameters"] = json!([]);
    spin["signature"]["result"] = json!("string:shared");
    for table in ["symbols", "accesses", "refs", "places", "values"] {
        spin[table] = json!([]);
    }
    spin["nodes"] = json!([
        {"id":"string:spin-loop", "kind":"loop", "original_kind":"expr_loop",
         "children":["string:spin-body"], "type":"string:never", "span":span,
         "provenance":{"origin":"source"}},
        {"id":"string:spin-body", "kind":"block", "original_kind":"expr_block",
         "children":[], "type":"type:unit", "span":span, "provenance":{"origin":"source"}}
    ]);
    spin["cfg"] = json!({"entry":"string:spin-loop", "exits":[], "edges":[]});
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(spin);
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"string:spin-definition", "kind":"function", "name":"spin_text",
            "stable_identity":"example::spin_text", "scope":"unit",
            "callable":{"parameters":[],"result":"string:shared","generics":0,"modifiers":[]}
        }));
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"string:spin-type", "kind":"function", "spelling":"fn() -> &str {spin_text}",
               "definition":"string:spin-definition", "parameters":[], "result":"string:shared"}),
        json!({"id":"string:never", "kind":"bottom", "spelling":"!"}),
    ]);
}

fn call_diverging_receiver(document: &mut Value) {
    let function = function(document, "count");
    let mut callee = node(function, "count.items").clone();
    callee["id"] = json!("string:spin-callee");
    callee["type"] = json!("string:spin-type");
    callee.as_object_mut().unwrap().remove("adjustments");
    function["nodes"].as_array_mut().unwrap().push(callee);
    let receiver = node(function, "count.items");
    receiver["kind"] = json!("call");
    receiver["original_kind"] = json!("expr_call");
    receiver["children"] = json!(["string:spin-callee"]);
    for reference in function["refs"].as_array_mut().unwrap().iter_mut() {
        if reference["node"] == "count.items" {
            reference["target"] = json!({"kind":"definition","id":"string:spin-definition"});
            reference["dispatch"] = json!("static");
        }
    }
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"string:spin-callee", "status":"resolved",
        "target":{"kind":"definition","id":"string:spin-definition"}
    }));
    for table in ["places", "accesses"] {
        function[table]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["node"] != "count.items");
    }
}

#[test]
fn a_computed_receiver_cannot_erase_its_nonterminating_evaluation() {
    let mut document = traversal("chars");
    add_diverging_string_function(&mut document);
    call_diverging_receiver(&mut document);
    // A loop with no guard of its own is opaque, so the spinning function has no bound.
    assert!(matches!(
        runtime_bounds::analyze(&program(document.clone(), "spin_text")),
        Outcome::Partial(assessment) if assessment.loops.iter().all(|bound| bound.opaque.is_some())
    ));
    unknown(document);
}

fn merge_callback_support(document: &mut Value, support: &Value) {
    for table in ["definitions", "types", "files"] {
        let entries = document["unit"][table].as_array_mut().unwrap();
        for entry in support["unit"][table].as_array().unwrap() {
            if !entries.iter().any(|existing| existing["id"] == entry["id"]) {
                entries.push(entry.clone());
            }
        }
    }
}

fn add_callback(document: &mut Value, mut support: Value, name: &str) -> Value {
    let mut callback = function(&mut support, name).clone();
    callback["signature"]["parameters"][0]["type"] = json!("string:char");
    callback["symbols"][0]["type"] = json!("string:char");
    let definition = document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == callback["definition"])
        .unwrap();
    definition["callable"]["parameters"] = json!(["string:char"]);
    let definition = definition.clone();
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"string:callback", "kind":"function", "spelling":"fn(char)",
               "definition":definition["id"], "parameters":["string:char"],
               "result":callback["signature"]["result"]}),
        json!({"id":"string:mapped", "kind":"opaque", "spelling":"Map<StringTraversal, fn(char)>"}),
    ]);
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(callback);
    definition
}

fn add_map(document: &mut Value, callback: &Value) {
    let function = function(document, "count");
    let span = node(function, "string:source-call")["span"].clone();
    function["nodes"].as_array_mut().unwrap().extend([
        json!({"id":"string:map-callback", "kind":"identifier", "original_kind":"expr_path",
               "children":[], "type":"string:callback", "span":span, "provenance":{"origin":"source"}}),
        json!({"id":"string:map", "kind":"call", "original_kind":"expr_method_call",
               "children":["string:source-call","string:map-callback"], "type":"string:mapped",
               "span":span, "provenance":{"origin":"source"}}),
    ]);
    function["refs"].as_array_mut().unwrap().extend([
        json!({"node":"string:map-callback", "status":"resolved",
               "target":{"kind":"definition", "id":callback["id"]}}),
        json!({"node":"string:map", "status":"resolved", "dispatch":"method", "item":"string:char",
               "target":{"kind":"definition", "id":"def:core::iter::traits::iterator::Iterator::map"},
               "instance_arguments":["type:iter",callback["callable"]["result"],"string:callback"],
               "calls_back":{"value":true,"reason":"Map runs the named callback once per item",
                             "runs":[{"kind":"definition","id":callback["id"]}]}}),
    ]);
    node(function, "count.into_iter")["children"] = json!(["string:map"]);
    node(function, "count.into_iter")["type"] = json!("string:mapped");
    node(function, "count.item_pat")["type"] = callback["callable"]["result"].clone();
    for id in ["count.iter_pat", "count.iter_ref"] {
        node(function, id)["type"] = json!("string:mapped");
    }
    for symbol in function["symbols"].as_array_mut().unwrap() {
        if symbol["id"] == "count.iter" {
            symbol["type"] = json!("string:mapped");
        } else if symbol["id"] == "count.item" {
            symbol["type"] = callback["callable"]["result"].clone();
        }
    }
}

fn mapped_scan(callback: &str) -> Value {
    let support: Value = serde_json::from_str(include_str!(
        "../src/cpg/testdata/for-iterator-chains.cpg.json"
    ))
    .unwrap();
    let mut document = traversal("chars");
    merge_callback_support(&mut document, &support);
    let definition = add_callback(&mut document, support, callback);
    add_map(&mut document, &definition);
    document
}

fn report_row(document: Value) -> Value {
    let demand = rstyle_core::cpg::records::Demand {
        bounds: true,
        ..Default::default()
    };
    let records = rstyle_core::cpg::run_records(vec![document], demand).unwrap();
    let report = serde_json::to_value(rstyle_core::cpg::bounds::report::from_records(
        &records, None,
    ))
    .unwrap();
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "count")
        .unwrap()
        .clone()
}

#[test]
fn string_adapter_callbacks_keep_invocations_and_composed_loop_work() {
    let document = mapped_scan("named_item");
    let assessment = proven(&program(document.clone(), "count"));
    assert_eq!(assessment.calls.len(), 1);
    assert!(assessment.calls[0].callee.contains("named_item"));
    for bytes in [0, 1, 11] {
        assert_eq!(
            assessment.calls[0].count.evaluate(&|_| Some(bytes)),
            Some(bytes)
        );
    }
    let row = report_row(document);
    let bound: Bound =
        serde_json::from_value(row["composed"]["Proven"]["loop_body_entries"].clone()).unwrap();
    assert!(bound.is_complete());
    // One string pull and the callback's three loop entries per source byte.
    assert_eq!(bound.evaluate(&|_| Some(11)), Some(44));
}

#[test]
fn string_sources_do_not_hide_owning_adapter_yield_cleanup() {
    unknown(mapped_scan("make_guard"));

    let mut missing_item = mapped_scan("named_item");
    let reference = function(&mut missing_item, "count")["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "string:map")
        .unwrap();
    reference.as_object_mut().unwrap().remove("item");
    unknown(missing_item);
}

#[test]
fn unresolved_string_adapter_callbacks_remain_unknown() {
    let mut document = mapped_scan("named_item");
    let reference = function(&mut document, "count")["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "string:map-callback")
        .unwrap();
    reference["status"] = json!("unresolved");
    reference.as_object_mut().unwrap().remove("target");
    unknown(document);
}

fn checksum(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[test]
fn the_saved_production_owner_has_an_actual_formal_byte_bound() {
    let document: Value = serde_json::from_str(PRODUCTION).unwrap();
    let provenance: Value = serde_json::from_str(PROVENANCE).unwrap();
    assert_eq!(
        checksum(PRODUCTION.as_bytes()),
        provenance["fixture_checksum"]
    );
    assert_eq!(
        checksum(&serde_json::to_vec(&document["unit"]["functions"][0]).unwrap()),
        provenance["owner_checksum"]
    );
    let program = program(document, "escape");
    let assessment = proven(&program);
    assert_eq!(program.extents.len(), 1);
    assert_eq!(program.extents[0].root, Root::Formal(0));
    assert!(program.extents[0].fields.is_empty());
    assert_eq!(
        assessment.loop_body_entries,
        Bound::Input(program.extents[0].input.clone())
    );
    assert_eq!(assessment.loop_body_entries.evaluate(&|_| Some(0)), Some(0));
    assert_eq!(
        assessment.loop_body_entries.evaluate(&|_| Some(11)),
        Some(11)
    );
}
