//! Exact owners from Bay's saved 2026-11-12 capture, pruned to their transitive facts.
//! Manifest SHA256: 32ec38acfdd06941bde7be76a82fd0b224d577c88e98e2c9df71c194b8f7293e.
//! Owners: go::github.com/commonfabric/bay/internal/engine.quoteArgs and
//! go::github.com/commonfabric/bay/internal/manifest.bayNotFoundError.
//! Original document SHA256s: 557d0900d43eaf8ab131715905992929c3187aeb566ff485549ef66970c426da
//! and 1340294cf03ccfc7e98543687fa5c8a685c514356b70a81f26bdbcdb9faec0a8.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::cpg::bounds::report::from_records;
use rstyle_core::cpg::records::Demand;
use rstyle_core::runtime_bounds::{self, Action, Bound, Outcome, Program};
use serde_json::{Value, json};

const QUOTE: &str = include_str!("testdata/native-output-quote-args-go.cpg.json");
const NOT_FOUND: &str = include_str!("testdata/native-output-bay-not-found-go.cpg.json");

fn quote() -> Value {
    serde_json::from_str(QUOTE).unwrap()
}

fn function(document: &mut Value) -> &mut Value {
    &mut document["unit"]["functions"][0]
}

fn part<'d>(document: &'d mut Value, id: &str) -> &'d mut Value {
    function(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn allocator(document: &mut Value) -> &mut Value {
    function(document)["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| {
            reference["target"]["id"] == "def:go::make" && reference["dispatch"] == "builtin"
        })
        .unwrap()
}

fn output(document: &mut Value) -> &mut Value {
    function(document)["symbols"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|symbol| symbol["name"] == "quoted")
        .unwrap()
}

fn first_store(document: &mut Value) -> String {
    let nodes = function(document)["nodes"].as_array().unwrap();
    let assignment = nodes
        .iter()
        .find(|node| node["kind"] == "assignment")
        .unwrap();
    assignment["children"][0].as_str().unwrap().to_owned()
}

fn add(document: &mut Value, id: &str, kind: &str, children: &[&str]) {
    let span = function(document)["span"].clone();
    function(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": id, "kind": kind, "original_kind": kind, "children": children,
            "span": span, "provenance": {"origin": "source"}
        }));
}

fn reference(document: &mut Value, node: &str, symbol: &str) {
    function(document)["refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node": node, "status": "resolved", "target": {"kind": "symbol", "id": symbol}
        }));
}

fn statement(document: &mut Value, id: &str) {
    add(document, "test.statement", "expression_statement", &[id]);
    let root = function(document)["root"].as_str().unwrap().to_owned();
    part(document, &root)["children"]
        .as_array_mut()
        .unwrap()
        .insert(1, json!("test.statement"));
}

fn program(document: Value) -> Program {
    let (_, input) = rstyle_core::cpg::run_bounds(vec![document])
        .unwrap()
        .remove(0);
    let Input::Captured { program } = input else {
        panic!("expected captured bounds: {input:?}");
    };
    program
}

fn scan(document: Value) -> Bound {
    let outcome = runtime_bounds::analyze(&program(document));
    let Outcome::Proven(assessment) = outcome else {
        panic!("expected a proven input scan: {outcome:?}");
    };
    assert_eq!(assessment.loops.len(), 1);
    assessment.loop_body_entries
}

fn unknown(document: Value) {
    let outcome = runtime_bounds::analyze(&program(document));
    assert!(matches!(outcome, Outcome::Unknown { .. }), "{outcome:?}");
}

#[test]
fn native_output_stores_recover_two_real_input_sized_scans() {
    assert_eq!(scan(quote()), Bound::Input("len(args)#0".into()));
    assert_eq!(
        scan(serde_json::from_str(NOT_FOUND).unwrap()),
        Bound::Input("len(hints)#2".into())
    );
}

#[test]
fn an_allocator_whose_definition_has_a_callable_keeps_its_type_operand() {
    // A builtin allocator's callable lists only its runtime parameters; the call still holds
    // the static type in argument zero, as it does without one.
    let mut document = quote();
    let definitions = document["unit"]["definitions"].as_array_mut().unwrap();
    let make = definitions
        .iter_mut()
        .find(|definition| definition["id"] == "def:go::make")
        .unwrap();
    make["callable"] =
        json!({"parameters": ["t1"], "result": "t1", "generics": 0, "modifiers": []});
    let program = program(document);
    let arguments = program
        .nodes
        .iter()
        .find_map(|node| match &node.action {
            Action::Call {
                callee, arguments, ..
            } if callee == "make" => Some(arguments.len()),
            _ => None,
        })
        .unwrap();
    assert_eq!(arguments, 2);
}

#[test]
fn allocation_and_length_calls_keep_their_costs_and_argument_positions() {
    let program = program(quote());
    let make = program
        .nodes
        .iter()
        .find_map(|node| match &node.action {
            Action::Call {
                callee, arguments, ..
            } if callee == "make" => Some(arguments),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        make.len(),
        2,
        "the static type still occupies argument zero"
    );
    assert!(program.nodes.iter().any(|node| matches!(
        &node.action, Action::Call { callee, .. } if callee == "len"
    )));
    let demand = Demand {
        bounds: true,
        graphs: false,
        flow: false,
        values: false,
        defects: false,
    };
    let records = rstyle_core::cpg::run_records(vec![quote()], demand).unwrap();
    let report = serde_json::to_value(from_records(&records, None)).unwrap();
    assert!(
        report["rows"][0]["composed"].get("Partial").is_some(),
        "{report}"
    );
}

#[test]
fn unresolved_dynamic_and_nonbuiltin_allocators_do_not_certify_storage() {
    for changed in ["unresolved", "dynamic", "static", "function_value"] {
        let mut document = quote();
        if changed == "unresolved" {
            allocator(&mut document)["status"] = json!("unresolved");
        } else {
            allocator(&mut document)["dispatch"] = json!(changed);
        }
        unknown(document);
    }
}

#[test]
fn operation_spelling_and_untrusted_definitions_do_not_make_a_native_allocator() {
    for changed in ["operation", "standard", "callee"] {
        let mut document = quote();
        if changed == "callee" {
            let call = allocator(&mut document)["node"]
                .as_str()
                .unwrap()
                .to_owned();
            let callee = part(&mut document, &call)["children"][0]
                .as_str()
                .unwrap()
                .to_owned();
            part(&mut document, &callee)["kind"] = json!("literal");
        } else {
            let definition = document["unit"]["definitions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|definition| definition["id"] == "def:go::make")
                .unwrap();
            definition[changed] = if changed == "operation" {
                json!("go:unknown")
            } else {
                json!(false)
            };
        }
        unknown(document);
    }
}

#[test]
fn type_operands_are_skipped_only_when_static_childless_and_correctly_typed() {
    for changed in ["kind", "children", "type"] {
        let mut document = quote();
        let call = allocator(&mut document)["node"]
            .as_str()
            .unwrap()
            .to_owned();
        let annotation = part(&mut document, &call)["children"][1]
            .as_str()
            .unwrap()
            .to_owned();
        match changed {
            "kind" => part(&mut document, &annotation)["kind"] = json!("block"),
            "type" => {
                part(&mut document, &annotation)
                    .as_object_mut()
                    .unwrap()
                    .remove("type");
            }
            _ => {
                add(&mut document, "test.type_work", "call", &[]);
                part(&mut document, &annotation)["children"] = json!(["test.type_work"]);
            }
        }
        unknown(document);
    }
}

#[test]
fn a_different_allocator_arity_remains_unknown() {
    let mut document = quote();
    let call = allocator(&mut document)["node"]
        .as_str()
        .unwrap()
        .to_owned();
    add(&mut document, "test.capacity", "literal", &[]);
    part(&mut document, &call)["children"]
        .as_array_mut()
        .unwrap()
        .push(json!("test.capacity"));
    unknown(document);
}

#[test]
fn mutable_or_rebound_headers_are_not_certified_fresh_storage() {
    let mut mutable = quote();
    output(&mut mutable)["mutable"] = json!(true);
    unknown(mutable);
    for shape in ["assignment", "tuple", "nested_tuple", "pattern_condition"] {
        let mut document = quote();
        let symbol = output(&mut document)["id"].as_str().unwrap().to_owned();
        let ty = output(&mut document)["type"].clone();
        add(&mut document, "test.rebound", "identifier", &[]);
        part(&mut document, "test.rebound")["type"] = ty.clone();
        reference(&mut document, "test.rebound", &symbol);
        add(&mut document, "test.replacement", "literal", &[]);
        part(&mut document, "test.replacement")["type"] = ty;
        let target = if matches!(shape, "tuple" | "nested_tuple") {
            add(&mut document, "test.targets", "tuple", &["test.rebound"]);
            if shape == "nested_tuple" {
                add(&mut document, "test.outer", "tuple", &["test.targets"]);
                "test.outer"
            } else {
                "test.targets"
            }
        } else {
            "test.rebound"
        };
        let kind = if shape == "pattern_condition" {
            shape
        } else {
            "assignment"
        };
        add(
            &mut document,
            "test.rebind",
            kind,
            &[target, "test.replacement"],
        );
        statement(&mut document, "test.rebind");
        unknown(document);
    }
}

#[test]
fn fresh_storage_requires_binding_identity_type_and_an_initializer() {
    for missing in ["node", "type", "initializer"] {
        let mut document = quote();
        if missing == "initializer" {
            let pattern = output(&mut document)["node"].clone();
            function(&mut document)["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|node| node["kind"] == "local_declaration" && node["children"][0] == pattern)
                .unwrap()["children"]
                .as_array_mut()
                .unwrap()
                .truncate(1);
        } else {
            output(&mut document)
                .as_object_mut()
                .unwrap()
                .remove(missing);
        }
        unknown(document);
    }
    for mode in ["shared", "exclusive"] {
        let mut document = quote();
        let pattern = output(&mut document)["node"].as_str().unwrap().to_owned();
        part(&mut document, &pattern)["binding_mode"] = json!(mode);
        unknown(document);
    }
}

#[test]
fn an_incomplete_sequence_layout_does_not_certify_native_allocation() {
    let mut document = quote();
    let ty = output(&mut document)["type"].clone();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == ty)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("fixed");
    unknown(document);
}

#[test]
fn a_nested_binding_with_the_same_name_does_not_rebind_the_output() {
    let mut document = quote();
    let mut shadow = output(&mut document).clone();
    shadow["id"] = json!("test.shadow.symbol");
    shadow["node"] = json!("test.shadow");
    shadow["scope"] = json!("test.scope");
    shadow["type"] = json!("t1");
    function(&mut document)["symbols"]
        .as_array_mut()
        .unwrap()
        .push(shadow);
    add(&mut document, "test.shadow", "pattern_binding", &[]);
    part(&mut document, "test.shadow")["type"] = json!("t1");
    add(&mut document, "test.empty", "literal", &[]);
    part(&mut document, "test.empty")["type"] = json!("t1");
    part(&mut document, "test.empty")["value"] = json!({"kind": "string", "value": ""});
    add(
        &mut document,
        "test.declaration",
        "local_declaration",
        &["test.shadow", "test.empty"],
    );
    add(&mut document, "test.scope", "block", &["test.declaration"]);
    statement(&mut document, "test.scope");
    assert_eq!(scan(document), Bound::Input("len(args)#0".into()));
}

#[test]
fn header_borrows_and_escape_places_remain_unknown() {
    let mut borrowed = quote();
    let symbol = output(&mut borrowed)["id"].as_str().unwrap().to_owned();
    add(&mut borrowed, "test.borrowed", "identifier", &[]);
    reference(&mut borrowed, "test.borrowed", &symbol);
    add(
        &mut borrowed,
        "test.borrow",
        "reference",
        &["test.borrowed"],
    );
    statement(&mut borrowed, "test.borrow");
    unknown(borrowed);
    for escapes in [json!(true), Value::Null] {
        let mut escaped = quote();
        function(&mut escaped)["places"][0]["escapes"] = escapes;
        unknown(escaped);
    }
}

#[test]
fn addressing_a_dereferenced_header_still_exposes_the_original_storage() {
    let mut document = quote();
    let symbol = output(&mut document)["id"].as_str().unwrap().to_owned();
    add(&mut document, "test.header", "identifier", &[]);
    reference(&mut document, "test.header", &symbol);
    add(&mut document, "test.pointer", "reference", &["test.header"]);
    add(&mut document, "test.deref", "operator", &["test.pointer"]);
    part(&mut document, "test.deref")["operator"] = json!("deref");
    add(&mut document, "test.address", "reference", &["test.deref"]);
    statement(&mut document, "test.address");
    let mut reordered = document.clone();
    function(&mut reordered)["nodes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    unknown(document);
    unknown(reordered);
}

#[test]
fn references_without_a_traceable_root_do_not_prove_private_storage() {
    let mut document = quote();
    add(&mut document, "test.temporary", "array", &[]);
    part(&mut document, "test.temporary")["type"] = output(&mut document)["type"].clone();
    add(
        &mut document,
        "test.address",
        "reference",
        &["test.temporary"],
    );
    statement(&mut document, "test.address");
    unknown(document);
}

#[test]
fn captures_and_missing_capture_rosters_do_not_certify_private_storage() {
    for roster in [None, Some(json!([])), Some(json!("captured"))] {
        let mut document = quote();
        add(&mut document, "test.nested", "item_declaration", &[]);
        if let Some(mut roster) = roster.clone() {
            if roster == "captured" {
                let symbol = output(&mut document)["id"].clone();
                let ty = output(&mut document)["type"].clone();
                roster = json!([{"symbol": symbol, "mode": "shared", "type": ty}]);
            }
            part(&mut document, "test.nested")["captures"] = roster;
        }
        let root = function(&mut document)["root"].as_str().unwrap().to_owned();
        part(&mut document, &root)["children"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!("test.nested"));
        if roster == Some(json!([])) {
            assert_eq!(scan(document), Bound::Input("len(args)#0".into()));
        } else {
            unknown(document);
        }
    }
}

#[test]
fn effectful_indices_and_computed_bases_stay_unknown() {
    for child in [0, 1] {
        let mut document = quote();
        let target = first_store(&mut document);
        let operand = part(&mut document, &target)["children"][child]
            .as_str()
            .unwrap()
            .to_owned();
        add(&mut document, "test.callee", "identifier", &[]);
        part(&mut document, &operand)["kind"] = json!("call");
        part(&mut document, &operand)["children"] = json!(["test.callee"]);
        unknown(document);
    }
}

#[test]
fn an_unmodeled_integer_expression_is_not_a_certified_pure_index() {
    let mut document = quote();
    let target = first_store(&mut document);
    let index = part(&mut document, &target)["children"][1]
        .as_str()
        .unwrap()
        .to_owned();
    part(&mut document, &index)["kind"] = json!("unsupported");
    unknown(document);
}

#[test]
fn unknown_or_overloaded_setters_and_compound_writes_stay_unknown() {
    for changed in ["overloaded", "dynamic", "compound_assignment"] {
        let mut document = quote();
        let target = first_store(&mut document);
        if changed == "compound_assignment" {
            let assignment = function(&mut document)["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|node| node["kind"] == "assignment")
                .unwrap();
            assignment["kind"] = json!(changed);
            assignment["operator"] = json!("add");
        } else {
            function(&mut document)["refs"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "node": target, "status": "unresolved", "dispatch": changed,
                    "reason": "an unknown setter may run"
                }));
        }
        unknown(document);
    }
}

#[test]
fn destructuring_native_output_stores_remain_unknown() {
    let mut document = quote();
    let target = first_store(&mut document);
    add(&mut document, "test.destructure", "tuple", &[&target]);
    let assignment = function(&mut document)["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["kind"] == "assignment")
        .unwrap();
    assignment["children"][0] = json!("test.destructure");
    unknown(document);
}
