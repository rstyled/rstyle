//! Exact Bay production capture, pruned to the owner's transitive facts.
//! Manifest SHA256: 32ec38acfdd06941bde7be76a82fd0b224d577c88e98e2c9df71c194b8f7293e.
//! document-000001 SHA256: 8a6c7f6c3ca7019c7e5ec8b8f5cd4c19a8d9c35ff71568157df12d0a934afd91.
//! Owner: go::github.com/commonfabric/bay/internal/cli.buildAgentPickerItems.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::{self, Action, Bound, Outcome, Program, Root, Value as Scalar};
use serde_json::{Value, json};

fn document() -> Value {
    serde_json::from_str(include_str!(
        "testdata/allocation-extents-picker-go.cpg.json"
    ))
    .unwrap()
}

fn function(document: &mut Value) -> &mut Value {
    &mut document["unit"]["functions"][0]
}

fn node<'d>(document: &'d mut Value, id: &str) -> &'d mut Value {
    function(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn symbol<'d>(document: &'d mut Value, name: &str) -> &'d mut Value {
    function(document)["symbols"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|symbol| symbol["name"] == name)
        .unwrap()
}

fn make(document: &mut Value) -> String {
    function(document)["refs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|reference| {
            reference["target"]["id"] == "def:go::make" && reference["dispatch"] == "builtin"
        })
        .unwrap()["node"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn length(document: &mut Value) -> String {
    let make = make(document);
    node(document, &make)["children"][2]
        .as_str()
        .unwrap()
        .to_owned()
}

fn program(document: Value) -> Program {
    let (_, input) = rstyle_core::cpg::run_bounds(vec![document])
        .unwrap()
        .remove(0);
    let Input::Captured { program } = input else {
        panic!("expected captured program: {input:?}");
    };
    program
}

fn bound(document: Value) -> Bound {
    let outcome = runtime_bounds::analyze(&program(document));
    let Outcome::Proven(assessment) = outcome else {
        panic!("expected the two production scans: {outcome:?}");
    };
    assert_eq!(assessment.loops.len(), 2);
    assessment.loop_body_entries
}

fn mixed(document: Value) {
    let bound = serde_json::to_string(&bound(document)).unwrap();
    assert!(bound.contains("len(sortedAll)#8"), "{bound}");
}

fn add(document: &mut Value, id: &str, kind: &str, children: &[&str]) {
    let span = function(document)["span"].clone();
    function(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": id, "kind": kind, "original_kind": kind, "span": span,
            "children": children, "provenance": {"origin": "source"}
        }));
}

fn reference(document: &mut Value, id: &str, name: &str) {
    add(document, id, "identifier", &[]);
    let target = symbol(document, name)["id"].clone();
    node(document, id)["type"] = symbol(document, name)["type"].clone();
    function(document)["refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"node":id,"status":"resolved","target":{"kind":"symbol","id":target}}));
}

fn statement(document: &mut Value, id: &str) {
    add(document, "test.statement", "expression_statement", &[id]);
    let root = function(document)["root"].as_str().unwrap().to_owned();
    node(document, &root)["children"]
        .as_array_mut()
        .unwrap()
        .insert(1, json!("test.statement"));
}

#[test]
fn production_allocation_turns_the_mixed_bound_into_two_formal_input_lengths() {
    assert_eq!(
        bound(document()),
        Bound::Add(vec![
            Bound::Input("len(available)#1".into()),
            Bound::Input("len(mru)#0".into()),
        ])
    );
    let program = program(document());
    assert!(program.extents.iter().any(|extent| {
        extent.input == "len(available)#1"
            && extent.root == Root::Formal(1)
            && extent.fields.is_empty()
    }));
    let limits: Vec<_> = program
        .nodes
        .iter()
        .filter_map(|node| match &node.action {
            Action::Assign { variable, value }
                if program.variables[*variable as usize]
                    .name
                    .starts_with("for-limit-") =>
            {
                Some(value)
            }
            _ => None,
        })
        .collect();
    assert!(limits.contains(&&Scalar::Input("len(available)#1".into())));
    assert!(!limits.contains(&&Scalar::Input("len(sortedAll)#8".into())));
}

#[test]
fn length_substitution_keeps_allocator_copy_sort_and_callback_calls() {
    let program = program(document());
    for expected in ["make", "len", "copy", "sort.Strings", "indirect call"] {
        assert!(
            program.nodes.iter().any(|node| matches!(
                &node.action, Action::Call { callee, .. } if callee == expected
            )),
            "missing {expected}"
        );
    }
    let arguments = program
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
        arguments.len(),
        2,
        "the static type keeps its original slot"
    );
}

#[test]
fn literal_allocation_lengths_are_exact_constants() {
    let mut document = document();
    let length = length(&mut document);
    let removed: Vec<_> = node(&mut document, &length)["children"]
        .as_array()
        .unwrap()
        .clone();
    node(&mut document, &length)["kind"] = json!("literal");
    node(&mut document, &length)["children"] = json!([]);
    node(&mut document, &length)["literal"] = json!("3");
    node(&mut document, &length)["value"] = json!({"kind":"integer","value":"3"});
    function(&mut document)["nodes"]
        .as_array_mut()
        .unwrap()
        .retain(|node| !removed.contains(&node["id"]));
    function(&mut document)["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != length && !removed.contains(&reference["node"]));
    assert_eq!(
        bound(document),
        Bound::Add(vec![Bound::Const(3), Bound::Input("len(mru)#0".into())])
    );
}

#[test]
fn unresolved_or_nonbuiltin_length_operations_keep_the_local_size_unknown() {
    for changed in ["unresolved", "dynamic", "static", "function_value"] {
        let mut document = document();
        let length = length(&mut document);
        let reference = function(&mut document)["refs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|reference| reference["node"] == length)
            .unwrap();
        if changed == "unresolved" {
            reference["status"] = json!(changed);
        } else {
            reference["dispatch"] = json!(changed);
        }
        mixed(document);
    }
}

#[test]
fn a_length_source_naming_a_symbol_the_body_lacks_supplies_no_relation() {
    // As an incomplete fact would: the relation is declined and the body is still bounded.
    let mut document = document();
    let length = length(&mut document);
    let source = node(&mut document, &length)["children"][1]
        .as_str()
        .unwrap()
        .to_owned();
    let reference = function(&mut document)["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == source)
        .unwrap();
    reference["target"]["id"] = json!("absent");
    mixed(document);
}

#[test]
fn mutable_formal_or_escaped_result_does_not_supply_an_exact_length_relation() {
    let mut mutable = document();
    symbol(&mut mutable, "available")["mutable"] = json!(true);
    mixed(mutable);
    let mut escaped = document();
    let output = symbol(&mut escaped, "sortedAll")["id"].clone();
    let place = function(&mut escaped)["places"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|place| place["root"]["symbol"] == output)
        .unwrap();
    place["escapes"] = json!(true);
    mixed(escaped);
}

#[test]
fn borrowing_either_header_invalidates_the_length_relation() {
    for header in ["available", "sortedAll"] {
        let mut document = document();
        reference(&mut document, "test.header", header);
        add(&mut document, "test.borrow", "reference", &["test.header"]);
        statement(&mut document, "test.borrow");
        mixed(document);
    }
}

#[test]
fn rebinding_either_header_prevents_a_complete_formal_input_proof() {
    for header in ["available", "sortedAll"] {
        let mut document = document();
        reference(&mut document, "test.header", header);
        reference(&mut document, "test.other", "mru");
        add(
            &mut document,
            "test.rebind",
            "assignment",
            &["test.header", "test.other"],
        );
        statement(&mut document, "test.rebind");
        assert!(matches!(
            runtime_bounds::analyze(&program(document)),
            Outcome::Unknown { .. }
        ));
    }
}

#[test]
fn an_unresolved_allocator_never_certifies_a_length_relation() {
    let mut document = document();
    let make = make(&mut document);
    function(&mut document)["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == make)
        .unwrap()["status"] = json!("unresolved");
    assert!(matches!(
        runtime_bounds::analyze(&program(document)),
        Outcome::Unknown { .. }
    ));
}

fn external_length_source(computed: bool) -> Value {
    let mut document = document();
    let length = length(&mut document);
    let source = node(&mut document, &length)["children"][1]
        .as_str()
        .unwrap()
        .to_owned();
    let ty = node(&mut document, &source)["type"].clone();
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"test:sequence", "kind":if computed { "function" } else { "static" },
            "name":"external_sequence", "scope":"library", "stable_identity":"test:sequence"
        }));
    let references = function(&mut document)["refs"].as_array_mut().unwrap();
    references.retain(|reference| reference["node"] != source);
    if computed {
        add(&mut document, "test.sequence-callee", "identifier", &[]);
        node(&mut document, &source)["kind"] = json!("call");
        node(&mut document, &source)["children"] = json!(["test.sequence-callee"]);
        function(&mut document)["refs"]
            .as_array_mut()
            .unwrap()
            .extend([
                json!({"node":"test.sequence-callee", "status":"resolved",
                   "target":{"kind":"definition","id":"test:sequence"}}),
                json!({"node":source, "status":"resolved", "dispatch":"static",
                   "target":{"kind":"definition","id":"test:sequence"}}),
            ]);
    } else {
        function(&mut document)["refs"].as_array_mut().unwrap().push(json!({
            "node":source,"status":"resolved","target":{"kind":"definition","id":"test:sequence"}
        }));
    }
    if computed {
        document["unit"]["types"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id":"test:factory-type","kind":"function","spelling":"fn() -> sequence",
                "definition":"test:sequence","parameters":[],"result":ty
            }));
        node(&mut document, "test.sequence-callee")["type"] = json!("test:factory-type");
    }
    document
}

#[test]
fn computed_or_global_length_sources_do_not_become_entry_formal_sizes() {
    for computed in [true, false] {
        let document = external_length_source(computed);
        if computed {
            let lowered = program(document.clone());
            assert!(lowered.nodes.iter().any(|node| matches!(
                &node.action, Action::Call { callee, .. } if callee == "external_sequence"
            )));
        }
        mixed(document);
    }
}
