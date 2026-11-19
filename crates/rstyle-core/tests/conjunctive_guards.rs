//! Core guard facts, independent of a frontend's syntax for a conjunction.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::{self, Assessment, Bound, Outcome, Program};
use serde_json::{Value, json};

fn expression(id: &str, kind: &str, ty: &str, children: &[&str]) -> Value {
    json!({"id": id, "kind": kind, "original_kind": kind, "type": ty,
           "children": children, "span": {"file": "file:lib", "start": 200, "end": 205},
           "provenance": {"origin": "source"}})
}

fn scalar(id: &str, value: &str) -> Value {
    let mut node = expression(id, "literal", "type:u32", &[]);
    node["value"] = json!({"kind": "integer", "value": value});
    node
}

fn operator(id: &str, op: &str, children: &[&str]) -> Value {
    let mut node = expression(id, "operator", "type:bool", children);
    node["operator"] = json!(op);
    node
}

fn document() -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    let function = count_function(&mut document);
    scan_signature(function);
    function["nodes"] = scan_nodes();
    node(function, "g:predicate")["value"] = json!({"kind": "bool", "value": "true"});
    node(function, "g:increment")["operator"] = json!("add");
    function["refs"] = json!([
        reference("g:read", "g:i"),
        reference("g:write", "g:i"),
        reference("g:limit-read", "g:limit")
    ]);
    for table in ["accesses", "places", "values"] {
        function[table] = json!([]);
    }
    function["cfg"] = json!({"entry": "entry", "exits": [], "edges": []});
    document
}

fn scan_signature(function: &mut Value) {
    function["root"] = json!("g:root");
    function["signature"]["parameters"] =
        json!([{"position": 0, "symbol": "g:limit", "type": "type:u32"}]);
    function["signature"]["result"] = json!("type:unit");
    function["symbols"] = json!([
        {"id": "g:limit", "name": "limit", "kind": "parameter", "scope": "fn:count",
         "type": "type:u32", "mutable": true,
         "declared": {"file": "file:lib", "start": 164, "end": 169}},
        {"id": "g:i", "name": "i", "kind": "local", "scope": "g:root",
         "type": "type:u32", "mutable": true, "node": "g:binding"}
    ]);
}

fn scan_nodes() -> Value {
    json!([
        expression("g:root", "block", "type:unit", &["g:let", "g:while"]),
        expression(
            "g:let",
            "local_declaration",
            "type:unit",
            &["g:binding", "g:zero"]
        ),
        expression("g:binding", "pattern_binding", "type:u32", &[]),
        scalar("g:zero", "0"),
        scalar("g:one", "1"),
        expression("g:read", "identifier", "type:u32", &[]),
        expression("g:limit-read", "identifier", "type:u32", &[]),
        operator("g:less", "lt", &["g:read", "g:limit-read"]),
        expression("g:predicate", "literal", "type:bool", &[]),
        operator("g:and", "and", &["g:less", "g:predicate"]),
        expression("g:while", "while", "type:unit", &["g:and", "g:body"]),
        expression("g:body", "block", "type:unit", &["g:increment"]),
        expression("g:write", "identifier", "type:u32", &[]),
        expression(
            "g:increment",
            "compound_assignment",
            "type:unit",
            &["g:write", "g:one"]
        )
    ])
}

fn reference(node: &str, symbol: &str) -> Value {
    json!({"node": node, "status": "resolved", "target": {"kind": "symbol", "id": symbol}})
}

fn count_function(document: &mut Value) -> &mut Value {
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

fn program(document: Value) -> Program {
    let (_, input) = rstyle_core::cpg::run_bounds(vec![document])
        .unwrap()
        .into_iter()
        .find(|(identity, _)| identity.ends_with("::count"))
        .unwrap();
    let Input::Captured { program } = input else {
        panic!("{input:?}");
    };
    program
}

fn proven(document: Value) -> Assessment {
    let result = runtime_bounds::analyze(&program(document));
    let Outcome::Proven(assessment) = result else {
        panic!("{result:?}");
    };
    assessment
}

fn unproven(document: Value) {
    let result = runtime_bounds::analyze(&program(document));
    assert!(!matches!(result, Outcome::Proven(_)), "{result:?}");
}

fn call_predicate(document: &mut Value, id: &str) {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": id, "kind": "function", "name": id, "stable_identity": id, "scope": "library",
            "callable": {"parameters": [], "result": "type:bool", "generics": 0, "modifiers": []}
        }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": format!("{id}-type"), "kind": "function", "spelling": id,
            "definition": id, "parameters": [], "result": "type:bool"
        }));
    let function = count_function(document);
    let callee = format!("{id}-callee");
    function["nodes"].as_array_mut().unwrap().push(expression(
        &callee,
        "identifier",
        &format!("{id}-type"),
        &[],
    ));
    *node(function, "g:predicate") = expression("g:predicate", "call", "type:bool", &[&callee]);
    function["refs"].as_array_mut().unwrap().extend([
        json!({"node": callee, "status": "resolved", "target": {"kind": "definition", "id": id}}),
        json!({"node": "g:predicate", "status": "resolved", "dispatch": "static",
               "target": {"kind": "definition", "id": id},
               "calls_back": {"value": false, "reason": "test predicate has no arguments"}}),
    ]);
}

#[test]
fn a_leading_input_guard_proves_a_real_scan() {
    let assessment = proven(document());
    assert_eq!(assessment.loops.len(), 1);
    assert!(matches!(assessment.loop_body_entries, Bound::Input(_)));
    assert_eq!(assessment.loop_body_entries.evaluate(&|_| Some(9)), Some(9));
}

#[test]
fn a_predicates_final_evaluation_is_counted_under_the_numeric_guard() {
    let mut document = document();
    call_predicate(&mut document, "test:predicate");
    let assessment = proven(document);
    assert_eq!(assessment.calls.len(), 1);
    assert_eq!(assessment.calls[0].count, assessment.loop_body_entries);
    assert_eq!(assessment.calls[0].count.evaluate(&|_| Some(0)), Some(0));
    assert_eq!(assessment.calls[0].count.evaluate(&|_| Some(5)), Some(5));
}

fn second_predicate(document: &mut Value) {
    call_predicate(document, "test:predicate");
    let function = count_function(document);
    let mut second = node(function, "g:predicate").clone();
    second["id"] = json!("g:second");
    second["children"] = json!(["g:second-callee"]);
    let mut callee = node(function, "test:predicate-callee").clone();
    callee["id"] = json!("g:second-callee");
    let mut reference = function["refs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|reference| reference["node"] == "g:predicate")
        .unwrap()
        .clone();
    reference["node"] = json!("g:second");
    function["refs"].as_array_mut().unwrap().extend([
        reference,
        json!({"node": "g:second-callee", "status": "resolved",
               "target": {"kind": "definition", "id": "test:predicate"}}),
    ]);
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .extend([second, callee]);
}

#[test]
fn later_short_circuit_predicates_keep_their_calls() {
    for nested_or in [false, true] {
        let mut document = document();
        second_predicate(&mut document);
        let function = count_function(&mut document);
        let outer = if nested_or {
            node(function, "g:and")["children"] = json!(["g:less", "g:outer"]);
            operator("g:outer", "or", &["g:predicate", "g:second"])
        } else {
            node(function, "g:while")["children"][0] = json!("g:outer");
            operator("g:outer", "and", &["g:and", "g:second"])
        };
        function["nodes"].as_array_mut().unwrap().push(outer);
        let assessment = proven(document);
        assert_eq!(assessment.calls.len(), 2);
        for call in assessment.calls {
            assert_eq!(call.count.evaluate(&|_| Some(7)), Some(7));
        }
    }
}

fn stable_extent() -> Value {
    let mut document = document();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "type:usize", "kind": "integer", "spelling": "usize", "key": "rust:usize",
            "bits": 64, "signed": false, "pointer_sized": true
        }));
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id": "test:length", "kind": "method", "name": "slice::len", "stable_identity": "slice::len",
        "scope": "library", "standard": true, "operation": "rust:slice.len",
        "callable": {"owner_type": "type:u32_slice", "receiver": "type:slice_ref", "parameters": [],
                     "result": "type:usize", "generics": 0, "modifiers": []}
    }));
    let function = count_function(&mut document);
    for node in function["nodes"].as_array_mut().unwrap() {
        if node["type"] == "type:u32" {
            node["type"] = json!("type:usize");
        }
    }
    for symbol in function["symbols"].as_array_mut().unwrap() {
        symbol["type"] = json!("type:usize");
    }
    function["signature"]["parameters"][0]["type"] = json!("type:usize");
    install_extent(function);
    document
}

fn install_extent(function: &mut Value) {
    function["symbols"].as_array_mut().unwrap().push(json!({
        "id": "g:items", "name": "items", "kind": "parameter", "scope": "fn:count",
        "type": "type:slice_ref", "mutable": false,
        "declared": {"file": "file:lib", "start": 164, "end": 169}
    }));
    function["signature"]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "position": 1, "symbol": "g:items", "type": "type:slice_ref"
        }));
    function["nodes"].as_array_mut().unwrap().push(expression(
        "g:items-read",
        "identifier",
        "type:slice_ref",
        &[],
    ));
    *node(function, "g:limit-read") =
        expression("g:limit-read", "call", "type:usize", &["g:items-read"]);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "g:limit-read");
    function["refs"].as_array_mut().unwrap().extend([
        reference("g:items-read", "g:items"),
        json!({"node": "g:limit-read", "status": "resolved", "dispatch": "method",
               "target": {"kind": "definition", "id": "test:length"}}),
    ]);
}

#[test]
fn a_resolved_stable_extent_can_bound_later_predicates() {
    let assessment = proven(stable_extent());
    assert!(
        matches!(&assessment.loop_body_entries, Bound::Input(input) if input.starts_with("len("))
    );
    assert_eq!(assessment.loop_body_entries.evaluate(&|_| Some(8)), Some(8));
}

fn wrap_guard(document: &mut Value, depth: usize) {
    let function = count_function(document);
    let mut inner = "g:and".to_owned();
    for index in 0..depth {
        let id = format!("g:condition-block-{index}");
        function["nodes"].as_array_mut().unwrap().push(expression(
            &id,
            "block",
            "type:bool",
            &[&inner],
        ));
        inner = id;
    }
    node(function, "g:while")["children"][0] = json!(inner);
}

#[test]
fn transparent_condition_blocks_preserve_the_input_proof() {
    let mut document = document();
    wrap_guard(&mut document, 3);
    assert_eq!(
        proven(document).loop_body_entries.evaluate(&|_| Some(6)),
        Some(6)
    );
}

#[test]
fn deeply_nested_condition_blocks_exceed_the_guard_certificate() {
    let mut document = document();
    wrap_guard(&mut document, 70);
    unproven(document);
}

#[test]
fn deeply_nested_boolean_spines_are_rejected_conservatively() {
    let mut document = document();
    let function = count_function(&mut document);
    let mut inner = "g:and".to_owned();
    for index in 0..70 {
        let id = format!("g:conjunct-{index}");
        let predicate = format!("g:conjunct-predicate-{index}");
        let mut value = expression(&predicate, "literal", "type:bool", &[]);
        value["value"] = json!({"kind": "bool", "value": "true"});
        function["nodes"]
            .as_array_mut()
            .unwrap()
            .extend([value, operator(&id, "and", &[&inner, &predicate])]);
        inner = id;
    }
    node(function, "g:while")["children"][0] = json!(inner);
    unproven(document);
}

fn nested_extent() -> Value {
    let mut document = stable_extent();
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"g:holder", "kind":"record", "spelling":"Holder",
               "fields":[{"name":"inner", "type":"g:inner"}]}),
        json!({"id":"g:inner", "kind":"record", "spelling":"Inner",
               "fields":[{"name":"rows", "type":"type:slice_ref"}]}),
        json!({"id":"g:holder-ref", "kind":"reference", "spelling":"&Holder",
               "mode":"shared", "target":"g:holder"}),
        json!({"id":"g:rows-ref", "kind":"reference", "spelling":"&&[u32]",
               "mode":"shared", "target":"type:slice_ref"}),
    ]);
    install_nested_extent(count_function(&mut document));
    document
}

fn install_nested_extent(function: &mut Value) {
    function["signature"]["parameters"][1]["type"] = json!("g:holder-ref");
    function["symbols"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|symbol| symbol["id"] == "g:items")
        .unwrap()["type"] = json!("g:holder-ref");
    function["nodes"].as_array_mut().unwrap().extend([
        expression("g:holder-read", "identifier", "g:holder-ref", &[]),
        expression("g:holder-deref", "operator", "g:holder", &["g:holder-read"]),
        expression(
            "g:inner-field",
            "field_access",
            "g:inner",
            &["g:holder-deref"],
        ),
        expression(
            "g:rows-borrow",
            "reference",
            "g:rows-ref",
            &["g:items-read"],
        ),
    ]);
    node(function, "g:holder-deref")["operator"] = json!("deref");
    node(function, "g:inner-field")["member"] = json!("inner");
    node(function, "g:inner-field")["member_index"] = json!(0);
    *node(function, "g:items-read") = expression(
        "g:items-read",
        "field_access",
        "type:slice_ref",
        &["g:inner-field"],
    );
    node(function, "g:items-read")["member"] = json!("rows");
    node(function, "g:items-read")["member_index"] = json!(0);
    node(function, "g:limit-read")["children"] = json!(["g:rows-borrow"]);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "g:items-read");
    function["refs"]
        .as_array_mut()
        .unwrap()
        .push(reference("g:holder-read", "g:items"));
}

#[test]
fn shared_nested_field_borrows_and_builtin_derefs_preserve_the_limit() {
    let assessment = proven(nested_extent());
    assert!(matches!(&assessment.loop_body_entries, Bound::Input(_)));
    assert_eq!(
        assessment.loop_body_entries.evaluate(&|_| Some(11)),
        Some(11)
    );
}

#[test]
fn extra_evaluation_in_a_length_projection_cannot_be_erased() {
    let mut document = nested_extent();
    let function = count_function(&mut document);
    function["nodes"].as_array_mut().unwrap().extend([
        expression("g:extra-loop", "loop", "type:unit", &["g:extra-body"]),
        expression("g:extra-body", "block", "type:unit", &[]),
    ]);
    node(function, "g:items-read")["children"]
        .as_array_mut()
        .unwrap()
        .push(json!("g:extra-loop"));
    unproven(document);
}

#[test]
fn deep_shared_reference_paths_exceed_the_guard_certificate() {
    let mut document = stable_extent();
    let mut inner = "g:items-read".to_owned();
    let mut ty = "type:slice_ref".to_owned();
    for index in 0..70 {
        let id = format!("g:borrow-{index}");
        let next_ty = format!("g:borrow-type-{index}");
        document["unit"]["types"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": next_ty, "kind":"reference", "spelling":"shared sequence view",
                "mode":"shared", "target":ty
            }));
        count_function(&mut document)["nodes"]
            .as_array_mut()
            .unwrap()
            .push(expression(&id, "reference", &next_ty, &[&inner]));
        inner = id;
        ty = next_ty;
    }
    node(count_function(&mut document), "g:limit-read")["children"] = json!([inner]);
    unproven(document);
}

#[test]
fn adjusted_condition_wrappers_do_not_supply_a_pure_numeric_header() {
    let mut document = document();
    wrap_guard(&mut document, 1);
    node(count_function(&mut document), "g:condition-block-0")["adjustments"] = json!([
        {"kind":"test:condition-conversion", "target":"type:bool"}
    ]);
    unproven(document);
}

#[test]
fn hidden_cleanup_in_a_length_place_cannot_disappear_from_the_guard() {
    let mut document = nested_extent();
    document["coverage"]["capabilities"]["implicit-calls"]["status"] = json!("unsupported");
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"g:holder-definition", "kind":"type", "name":"Holder",
            "stable_identity":"test::Holder", "scope":"unit", "destructor":true
        }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["id"] == "g:holder")
        .unwrap()["definition"] = json!("g:holder-definition");
    unproven(document);
}

#[test]
fn a_computed_extent_receiver_cannot_discard_its_call() {
    let mut document = stable_extent();
    call_predicate(&mut document, "test:factory");
    for definition in document["unit"]["definitions"].as_array_mut().unwrap() {
        if definition["id"] == "test:factory" {
            definition["callable"]["result"] = json!("type:slice_ref");
        }
    }
    for ty in document["unit"]["types"].as_array_mut().unwrap() {
        if ty["id"] == "test:factory-type" {
            ty["result"] = json!("type:slice_ref");
        }
    }
    let function = count_function(&mut document);
    *node(function, "g:items-read") = expression(
        "g:items-read",
        "call",
        "type:slice_ref",
        &["test:factory-callee"],
    );
    *node(function, "g:predicate") = expression("g:predicate", "literal", "type:bool", &[]);
    node(function, "g:predicate")["value"] = json!({"kind": "bool", "value": "true"});
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "g:items-read");
    for reference in function["refs"].as_array_mut().unwrap() {
        if reference["node"] == "g:predicate" {
            reference["node"] = json!("g:items-read");
        }
    }
    unproven(document);
}

#[test]
fn predicate_writes_cannot_reset_the_counter_or_change_the_limit() {
    for symbol in ["g:i", "g:limit"] {
        let mut document = document();
        let function = count_function(&mut document);
        let target = expression("g:predicate-write", "identifier", "type:u32", &[]);
        let assignment = expression(
            "g:reset",
            "assignment",
            "type:unit",
            &["g:predicate-write", "g:reset-zero"],
        );
        let tail = node(function, "g:predicate").clone();
        function["nodes"].as_array_mut().unwrap().extend([
            target,
            assignment,
            expression(
                "g:reset-stmt",
                "expression_statement",
                "type:unit",
                &["g:reset"],
            ),
            scalar("g:reset-zero", "0"),
            {
                let mut tail = tail;
                tail["id"] = json!("g:true");
                tail
            },
        ]);
        *node(function, "g:predicate") = expression(
            "g:predicate",
            "block",
            "type:bool",
            &["g:reset-stmt", "g:true"],
        );
        function["refs"]
            .as_array_mut()
            .unwrap()
            .push(reference("g:predicate-write", symbol));
        unproven(document);
    }
}

#[test]
fn continue_and_break_preserve_the_counter_ranking() {
    let mut document = document();
    let function = count_function(&mut document);
    let mut predicate = node(function, "g:predicate").clone();
    predicate["id"] = json!("g:body-predicate");
    function["nodes"].as_array_mut().unwrap().extend([
        predicate,
        expression("g:continue", "continue", "type:unit", &[]),
        expression("g:break", "break", "type:unit", &[]),
        expression(
            "g:increment-stmt",
            "expression_statement",
            "type:unit",
            &["g:increment"],
        ),
        expression("g:if-stmt", "expression_statement", "type:unit", &["g:if"]),
        expression(
            "g:if",
            "if",
            "type:unit",
            &["g:body-predicate", "g:continue", "g:break"],
        ),
    ]);
    node(function, "g:body")["children"] = json!(["g:increment-stmt", "g:if-stmt"]);
    assert_eq!(proven(document.clone()).loops.len(), 1);
    node(count_function(&mut document), "g:body")["children"] =
        json!(["g:if-stmt", "g:increment-stmt"]);
    unproven(document);
}

#[test]
fn only_a_pure_leftmost_less_guard_is_selected() {
    let mut rightmost = document();
    node(count_function(&mut rightmost), "g:and")["children"] = json!(["g:predicate", "g:less"]);
    unproven(rightmost);
    let mut disjunction = document();
    node(count_function(&mut disjunction), "g:and")["operator"] = json!("or");
    unproven(disjunction);
    let mut adjusted = document();
    node(count_function(&mut adjusted), "g:read")["adjustments"] = json!([
        {"kind": "auto_ref", "mode": "shared", "target": "type:u32_ref"}
    ]);
    unproven(adjusted);
}

#[test]
fn signed_and_wrapping_counters_keep_the_existing_no_overflow_proof() {
    let mut document = document();
    document["unit"]["semantics"]["integer_overflow"] = json!("wrapping");
    let ty = document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["id"] == "type:u32")
        .unwrap();
    ty["signed"] = json!(true);
    let assessment = proven(document.clone());
    assert!(matches!(&assessment.loop_body_entries, Bound::Max(parts)
        if parts.contains(&Bound::Const(0))));
    assert_eq!(
        assessment.loop_body_entries.evaluate(&|_| Some(12)),
        Some(12)
    );
    node(count_function(&mut document), "g:zero")["value"]["value"] = json!("-1");
    unproven(document);
}
