//! Source calls are iterable values, not compiler-generated wrappers around their last argument.
//! These generic profile facts do not grant any operation an iteration/extent contract.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::{self, Bound, Outcome};
use serde_json::{Value, json};

fn document() -> Value {
    serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap()
}

fn count(document: &mut Value) -> &mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "count")
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

fn outcome(document: Value, name: &str) -> Outcome {
    let programs = rstyle_core::cpg::run_bounds(vec![document]).unwrap();
    let (_, input) = programs
        .into_iter()
        .find(|(identity, _)| identity.ends_with(name))
        .unwrap();
    let Input::Captured { program } = input else {
        panic!("expected captured input: {input:?}");
    };
    runtime_bounds::analyze(&program)
}

fn unknown(document: Value) {
    let result = outcome(document, "::count");
    assert!(matches!(result, Outcome::Unknown { .. }), "{result:?}");
}

/// `for item in unknown_iterator(items)`, with a resolved ordinary function reference.
fn source_call() -> Value {
    let mut document = document();
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id": "test:iterator", "kind": "function", "name": "unknown_iterator",
        "stable_identity": "test::unknown_iterator", "scope": "library",
        "callable": {"parameters": ["type:slice_ref"], "result": "type:iter", "generics": 0, "modifiers": []}
    }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "test:iterator-type", "kind": "function", "spelling": "fn(items) -> Iterator",
            "definition": "test:iterator", "parameters": ["type:slice_ref"], "result": "type:iter"
        }));
    let function = count(&mut document);
    let mut callee = node(function, "count.items").clone();
    callee["id"] = json!("test:callee");
    callee["type"] = json!("test:iterator-type");
    function["nodes"].as_array_mut().unwrap().push(callee);
    let call = node(function, "count.into_iter");
    call["synthetic"] = json!(false);
    call["provenance"] = json!({"origin": "source"});
    call["children"] = json!(["test:callee", "count.items"]);
    let refs = function["refs"].as_array_mut().unwrap();
    let call_ref = refs
        .iter_mut()
        .find(|reference| reference["node"] == "count.into_iter")
        .unwrap();
    call_ref["target"] = json!({"kind": "definition", "id": "test:iterator"});
    refs.push(json!({"node": "test:callee", "status": "resolved",
                     "target": {"kind": "definition", "id": "test:iterator"}}));
    document
}

fn with_spinning_argument() -> Value {
    let mut document = source_call();
    let spin = spinning_function(count(&mut document).clone());
    let span = spin["span"].clone();
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(spin);
    add_spinning_declaration(&mut document);
    for definition in document["unit"]["definitions"].as_array_mut().unwrap() {
        if definition["id"] == "test:iterator" {
            definition["callable"]["parameters"] = json!(["type:unit", "type:slice_ref"]);
        }
    }
    for ty in document["unit"]["types"].as_array_mut().unwrap() {
        if ty["id"] == "test:iterator-type" {
            ty["parameters"] = json!(["type:unit", "type:slice_ref"]);
        }
    }
    add_argument_call(count(&mut document), span);
    document
}

fn spinning_function(mut spin: Value) -> Value {
    let span = spin["span"].clone();
    spin["id"] = json!("test:spin-body");
    spin["definition"] = json!("test:spin");
    spin["name"] = json!("spin");
    spin["root"] = json!("test:spin-loop");
    spin["signature"]["parameters"] = json!([]);
    spin["signature"]["result"] = json!("type:unit");
    for table in ["symbols", "accesses", "refs", "places", "values"] {
        spin[table] = json!([]);
    }
    spin["nodes"] = json!([
        {"id": "test:spin-loop", "kind": "loop", "original_kind": "loop", "children": ["test:spin-block"],
         "type": "type:unit", "provenance": {"origin": "source"}, "span": span},
        {"id": "test:spin-block", "kind": "block", "original_kind": "block", "children": [],
         "type": "type:unit", "provenance": {"origin": "source"}, "span": span}
    ]);
    spin["cfg"] = json!({"entry": "test:spin-entry", "exits": [], "edges": []});
    spin
}

fn add_spinning_declaration(document: &mut Value) {
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id": "test:spin", "kind": "function", "name": "spin", "stable_identity": "test::spin", "scope": "unit",
        "callable": {"parameters": [], "result": "type:unit", "generics": 0, "modifiers": []}
    }));
    document["unit"]["types"].as_array_mut().unwrap().push(json!({
        "id": "test:spin-type", "kind": "function", "spelling": "fn() -> ()", "definition": "test:spin",
        "parameters": [], "result": "type:unit"
    }));
}

fn add_argument_call(function: &mut Value, span: Value) {
    function["nodes"].as_array_mut().unwrap().extend([
        json!({"id": "test:spin-callee", "kind": "identifier", "original_kind": "identifier", "children": [],
               "type": "test:spin-type", "provenance": {"origin": "source"}, "span": span}),
        json!({"id": "test:spin-call", "kind": "call", "original_kind": "call", "children": ["test:spin-callee"],
               "type": "type:unit", "provenance": {"origin": "source"}, "span": span})
    ]);
    function["refs"].as_array_mut().unwrap().extend([
        json!({"node": "test:spin-callee", "status": "resolved", "target": {"kind": "definition", "id": "test:spin"}}),
        json!({"node": "test:spin-call", "status": "resolved", "dispatch": "static",
               "target": {"kind": "definition", "id": "test:spin"}})
    ]);
    node(function, "count.into_iter")["children"] =
        json!(["test:callee", "test:spin-call", "count.items"]);
}

#[test]
fn the_generated_for_wrapper_keeps_the_written_sequence_scan() {
    let result = outcome(document(), "::count");
    let Outcome::Proven(assessment) = result else {
        panic!("{result:?}");
    };
    assert!(!assessment.loops.is_empty());
    assert!(matches!(assessment.loop_body_entries, Bound::Input(_)));
}

#[test]
fn an_ordinary_source_call_does_not_inherit_its_last_arguments_extent() {
    unknown(source_call());
}

#[test]
fn a_source_calls_nonterminating_argument_cannot_disappear() {
    let document = with_spinning_argument();
    assert!(matches!(
        outcome(document.clone(), "::spin"),
        Outcome::Unknown { .. }
    ));
    unknown(document);
}

#[test]
fn unresolved_source_calls_do_not_inherit_argument_extents_either() {
    let mut document = source_call();
    let reference = count(&mut document)["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "count.into_iter")
        .unwrap();
    reference["status"] = json!("unresolved");
    reference.as_object_mut().unwrap().remove("target");
    unknown(document);
}

#[test]
fn a_live_growing_iterator_has_no_entry_length_contract() {
    let mut document = source_call();
    document["unit"]["language"]["name"] = json!("python");
    let function = count(&mut document);
    node(function, "count.items")["type"] = json!("type:u32_slice");
    function["symbols"][0]["type"] = json!("type:u32_slice");
    function["signature"]["parameters"][0]["type"] = json!("type:u32_slice");
    // A resolved factory can return a live iterator and grow the list through an alias.
    // No emitted fact says its number of yields is the list's entry length.
    unknown(document);
}

#[test]
fn source_calls_stay_whole_inside_a_real_generated_for_wrapper() {
    let mut document = source_call();
    let function = count(&mut document);
    let mut wrapper = node(function, "count.into_iter").clone();
    wrapper["id"] = json!("test:generated");
    wrapper["synthetic"] = json!(true);
    wrapper["provenance"] = json!({"origin": "desugaring", "construct": "for"});
    wrapper["children"] = json!(["count.into_iter"]);
    function["nodes"].as_array_mut().unwrap().push(wrapper);
    node(function, "count.for")["children"][1] = json!("test:generated");
    unknown(document);
}

#[test]
fn wrapper_peeling_requires_all_the_compiler_for_provenance_facts() {
    for (synthetic, origin, construct) in [
        (false, "desugaring", "for"),
        (true, "source", "for"),
        (true, "desugaring", "while"),
    ] {
        let mut document = document();
        let wrapper = node(count(&mut document), "count.into_iter");
        wrapper["synthetic"] = json!(synthetic);
        wrapper["provenance"] = json!({"origin": origin, "construct": construct});
        unknown(document);
    }
}

#[test]
fn a_wrapper_with_several_source_children_does_not_discard_evaluation() {
    let mut document = document();
    let function = count(&mut document);
    let mut extra = node(function, "count.items").clone();
    extra["id"] = json!("test:extra");
    extra["kind"] = json!("loop");
    extra["type"] = json!("type:unit");
    function["nodes"].as_array_mut().unwrap().push(extra);
    node(function, "count.into_iter")["children"] = json!(["test:extra", "count.items"]);
    unknown(document);
}

#[test]
fn a_direct_sequence_source_needs_no_generated_wrapper() {
    let mut document = document();
    let function = count(&mut document);
    node(function, "count.into_iter")["children"] = json!([]);
    node(function, "count.for")["children"][1] = json!("count.items");
    let result = outcome(document, "::count");
    assert!(matches!(result, Outcome::Proven(_)), "{result:?}");
}
