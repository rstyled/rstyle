//! Summary substitution must use the parameter passed, even when it is written last.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::compose::{Analyzed, Composed, Composer};
use rstyle_core::runtime_bounds::{self, Action, Argument, Bound, Flow, Limits, Outcome, Program};
use rstyle_core::wording::Wording;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn example() -> Value {
    serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap()
}

fn expression(id: &str, kind: &str, children: &[&str]) -> Value {
    let ty = match kind {
        "block" | "local_declaration" | "while" | "compound_assignment" | "call" => "type:unit",
        "operator" => "type:bool",
        _ => "type:u32",
    };
    json!({"id":id, "kind":kind, "original_kind":kind, "children":children,
           "type":ty, "span":{"file":"file:lib","start":200,"end":205},
           "provenance":{"origin":"source"}})
}

fn literal(id: &str, number: u32) -> Value {
    let mut node = expression(id, "literal", &[]);
    node["value"] = json!({"kind":"integer", "value":number.to_string()});
    node
}

fn reference(node: &str, kind: &str, target: &str) -> Value {
    json!({"node":node, "status":"resolved", "target":{"kind":kind,"id":target}})
}

fn parameter(id: &str) -> Value {
    json!({"id":id, "name":id, "kind":"parameter", "scope":"test",
           "type":"type:u32", "mutable":false,
           "declared":{"file":"file:lib","start":164,"end":169}})
}

fn blank(name: &str) -> Value {
    let mut function = example()["unit"]["functions"][2].clone();
    function["id"] = json!(format!("fn:{name}"));
    function["definition"] = json!(format!("def:{name}"));
    function["name"] = json!(name);
    function["root"] = json!(format!("{name}.root"));
    function["signature"]["parameters"] = json!([]);
    function["signature"]["result"] = json!("type:unit");
    for table in ["nodes", "symbols", "refs", "accesses", "places", "values"] {
        function[table] = json!([]);
    }
    function["cfg"] = json!({"entry":"entry", "exits":[], "edges":[]});
    function
}

fn scan() -> Value {
    let mut function = blank("scan");
    function["signature"]["parameters"] = json!([
        {"position":0,"symbol":"limit","type":"type:u32"},
        {"position":1,"symbol":"unused","type":"type:u32"}
    ]);
    function["symbols"] = json!([
        parameter("limit"), parameter("unused"),
        {"id":"i","name":"i","kind":"local","scope":"scan.root","type":"type:u32",
         "mutable":true,"node":"scan.pattern"}
    ]);
    function["nodes"] = json!([
        expression("scan.root", "block", &["scan.let", "scan.while"]),
        expression(
            "scan.let",
            "local_declaration",
            &["scan.pattern", "scan.zero"]
        ),
        expression("scan.pattern", "pattern_binding", &[]),
        literal("scan.zero", 0),
        literal("scan.one", 1),
        expression("scan.while", "while", &["scan.less", "scan.body"]),
        expression("scan.less", "operator", &["scan.i", "scan.limit"]),
        expression("scan.i", "identifier", &[]),
        expression("scan.limit", "identifier", &[]),
        expression("scan.body", "block", &["scan.increment"]),
        expression(
            "scan.increment",
            "compound_assignment",
            &["scan.write", "scan.one"]
        ),
        expression("scan.write", "identifier", &[])
    ]);
    node(&mut function, "scan.less")["operator"] = json!("lt");
    node(&mut function, "scan.increment")["operator"] = json!("add");
    function["refs"] = json!([
        reference("scan.i", "symbol", "i"),
        reference("scan.write", "symbol", "i"),
        reference("scan.limit", "symbol", "limit")
    ]);
    function
}

fn caller() -> Value {
    let mut function = blank("caller");
    function["signature"]["parameters"] = json!([{"position":0,"symbol":"n","type":"type:u32"}]);
    function["symbols"] = json!([parameter("n")]);
    function["nodes"] = json!([
        expression("caller.root", "block", &["caller.call"]),
        expression(
            "caller.call",
            "call",
            &["caller.callee", "caller.unused", "caller.limit"]
        ),
        expression("caller.callee", "identifier", &[]),
        expression("caller.unused", "extension", &["caller.zero"]),
        literal("caller.zero", 0),
        expression("caller.limit", "extension", &["caller.n"]),
        expression("caller.n", "identifier", &[])
    ]);
    node(&mut function, "caller.callee")["type"] = json!("type:scan");
    for id in ["caller.unused", "caller.limit"] {
        node(&mut function, id)["extension"] = json!("py:keyword");
    }
    let mut call = reference("caller.call", "definition", "def:scan");
    call["dispatch"] = json!("static");
    call["arguments"] = json!([
        {"position":1,"parameter":"unused","value":"caller.zero"},
        {"position":0,"parameter":"limit","value":"caller.n"}
    ]);
    function["refs"] = json!([
        call,
        reference("caller.callee", "definition", "def:scan"),
        reference("caller.n", "symbol", "n")
    ]);
    function
}

fn document() -> Value {
    let mut document = example();
    document["unit"]["functions"] = json!([scan(), caller()]);
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!({"id":"def:scan","kind":"function","name":"scan","stable_identity":"scan",
               "scope":"unit","compilation":"test",
               "callable":{"parameters":["type:u32","type:u32"],"result":"type:unit",
                           "generics":0,"modifiers":[]}}),
            json!({"id":"def:caller","kind":"function","name":"caller","stable_identity":"caller",
               "scope":"unit","compilation":"test"}),
        ]);
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"type:scan","kind":"function","spelling":"scan","definition":"def:scan",
            "parameters":["type:u32","type:u32"],"result":"type:unit"
        }));
    document
}

fn node<'a>(function: &'a mut Value, id: &str) -> &'a mut Value {
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn bindings(document: &mut Value) -> &mut Value {
    &mut document["unit"]["functions"][1]["refs"][0]["arguments"]
}

fn callable(document: &mut Value) -> &mut Value {
    &mut document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "def:scan")
        .unwrap()["callable"]
}

fn direct_arguments(document: &mut Value) {
    let caller = &mut document["unit"]["functions"][1];
    node(caller, "caller.call")["children"] = json!(["caller.callee", "caller.zero", "caller.n"]);
    caller["nodes"]
        .as_array_mut()
        .unwrap()
        .retain(|node| node["id"] != "caller.unused" && node["id"] != "caller.limit");
}

fn programs(document: Value) -> BTreeMap<String, Program> {
    rstyle_core::cpg::run_bounds(vec![document])
        .unwrap()
        .into_iter()
        .map(|(id, input)| {
            let Input::Captured { program } = input else {
                panic!("{id}: {input:?}")
            };
            (format!("test/{id}"), program)
        })
        .collect()
}

fn compose(document: Value) -> Composed {
    let programs = programs(document);
    let mut composer = Composer::new(
        |id: &str| {
            programs.get(id).map(|program| Analyzed {
                program: program.clone(),
                outcome: runtime_bounds::analyze(program),
            })
        },
        Limits::default(),
    );
    composer.composed("test/caller", Wording::of("rust"))
}

fn proven_count(document: Value) -> Bound {
    let composed = compose(document);
    let Composed::Proven(composition) = composed else {
        panic!("{composed:?}")
    };
    assert_eq!(composition.workspace_invocations, Bound::Const(1));
    composition.loop_body_entries
}

fn assert_partial(document: Value) {
    let composed = compose(document);
    let Composed::Partial(composition) = composed else {
        panic!("{composed:?}")
    };
    assert!(
        !composition.loop_body_entries.is_complete(),
        "{composition:?}"
    );
    assert_eq!(composition.workspace_invocations, Bound::Const(1));
    assert!(
        composition
            .opaque_factors
            .iter()
            .any(|factor| factor.reason.contains("scalar argument"))
    );
}

#[test]
fn a_reordered_limit_never_composes_to_the_zero_written_first() {
    let count = proven_count(document());
    assert_eq!(count, Bound::Input("n".into()));
    assert_eq!(
        count.evaluate(&|name| (name == "n").then_some(37)),
        Some(37)
    );
}

#[test]
fn a_fixed_positional_call_still_substitutes_its_actual_limit() {
    let mut document = document();
    direct_arguments(&mut document);
    let caller = &mut document["unit"]["functions"][1];
    node(caller, "caller.call")["children"] = json!(["caller.callee", "caller.n", "caller.zero"]);
    caller["refs"][0]
        .as_object_mut()
        .unwrap()
        .remove("arguments");
    assert_eq!(proven_count(document), Bound::Input("n".into()));
}

#[test]
fn a_default_hole_has_no_argument_size_instead_of_the_next_written_value() {
    let mut document = document();
    node(&mut document["unit"]["functions"][1], "caller.call")["children"] =
        json!(["caller.callee", "caller.unused"]);
    *bindings(&mut document) = json!([{"position":1,"value":"caller.zero"}]);
    assert_partial(document);
}

#[test]
fn unknown_or_conflicting_positions_supply_no_formal_sizes() {
    for positions in [
        json!([{}, {"position":0}]),
        json!([{"position":0},{"position":0}]),
        json!([{"position":usize::MAX},{"position":0}]),
    ] {
        let mut document = document();
        let entries = bindings(&mut document).as_array_mut().unwrap();
        for (entry, position) in entries.iter_mut().zip(positions.as_array().unwrap()) {
            entry.as_object_mut().unwrap().remove("position");
            entry
                .as_object_mut()
                .unwrap()
                .extend(position.as_object().unwrap().clone());
        }
        assert_partial(document);
    }
}

#[test]
fn a_spreads_first_position_is_not_one_bound_argument() {
    for spread in ["sequence", "mapping"] {
        let mut document = document();
        direct_arguments(&mut document);
        *bindings(&mut document) = if spread == "sequence" {
            json!([{"spread":spread,"position":0}, {"position":1}])
        } else {
            json!([{"spread":spread}, {"position":1}])
        };
        assert_partial(document);
    }
}

#[test]
fn variadic_and_unavailable_interfaces_cannot_certify_formal_positions() {
    for absent in [false, true] {
        for mapped in [false, true] {
            let mut document = document();
            if absent {
                *callable(&mut document) = Value::Null;
            } else {
                callable(&mut document)["modifiers"] = json!(["variadic"]);
            }
            if !mapped {
                document["unit"]["functions"][1]["refs"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("arguments");
                direct_arguments(&mut document);
            }
            assert_partial(document);
        }
    }
}

#[test]
fn a_variadic_definition_cannot_bind_a_collected_value_to_a_later_defaulted_formal() {
    // `scan(*unused, limit=100)` called as `scan(0, n)` collects both written values.
    // The second value is not the defaulted `limit`, even if callable modifiers omit variadic.
    let mut document = document();
    direct_arguments(&mut document);
    document["unit"]["functions"][1]["refs"][0]
        .as_object_mut()
        .unwrap()
        .remove("arguments");
    let scan = &mut document["unit"]["functions"][0];
    scan["signature"]["parameters"] = json!([
        {"position":0,"symbol":"unused","type":"type:slice_ref"},
        {"position":1,"symbol":"limit","type":"type:u32"}
    ]);
    scan["signature"]["modifiers"] = json!(["variadic"]);
    scan["symbols"][1]["type"] = json!("type:slice_ref");
    callable(&mut document)["parameters"] = json!(["type:slice_ref", "type:u32"]);
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "def:scan")
        .unwrap()["modifiers"] = json!(["variadic"]);
    assert_partial(document);
}

#[test]
fn an_unbounded_interface_roster_is_never_allocated() {
    let mut document = document();
    callable(&mut document)["parameters"] = json!(vec!["type:u32"; 1025]);
    assert_partial(document);
}

fn call_arguments(document: Value) -> Vec<Argument> {
    programs(document)["test/caller"]
        .nodes
        .iter()
        .find_map(|node| match &node.action {
            Action::Call { arguments, .. } => Some(arguments.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn an_uncertified_interface_keeps_original_slots_without_assigning_them_values() {
    let mut document = document();
    *callable(&mut document) = Value::Null;
    assert_eq!(call_arguments(document), vec![Argument::Opaque; 2]);
}

fn selected_interface(document: &mut Value) {
    document["unit"]["definitions"].as_array_mut().unwrap().extend([
        json!({"id":"def:Interface","kind":"trait","name":"Interface",
               "stable_identity":"Interface","scope":"unit"}),
        json!({"id":"def:interface","kind":"method","name":"interface",
               "stable_identity":"interface","scope":"unit","owner":"def:Interface",
               "callable":{"parameters":["type:u32","type:u32"],"result":"type:unit","generics":0}}),
    ]);
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"type:interface","kind":"function","spelling":"interface",
            "definition":"def:interface","parameters":["type:u32","type:u32"],"result":"type:unit"
        }));
    let caller = &mut document["unit"]["functions"][1];
    node(caller, "caller.callee")["type"] = json!("type:interface");
    caller["refs"][1]["target"]["id"] = json!("def:interface");
    caller["refs"][0]["target"]["id"] = json!("def:interface");
    caller["refs"][0]["selected"] = json!({"kind":"definition","id":"def:scan"});
}

#[test]
fn the_selected_implementations_interface_certifies_its_formal_positions() {
    let mut document = document();
    selected_interface(&mut document);
    assert_eq!(proven_count(document), Bound::Input("n".into()));
}

#[test]
fn positive_variadic_facts_on_either_selected_or_declared_definition_reject_binding() {
    for id in ["def:interface", "def:scan"] {
        let mut document = document();
        selected_interface(&mut document);
        document["unit"]["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|definition| definition["id"] == id)
            .unwrap()["modifiers"] = json!(["variadic"]);
        assert_partial(document);
    }
}

#[test]
fn dependent_selection_supplies_no_certified_argument_sizes() {
    let mut document = document();
    selected_interface(&mut document);
    document["unit"]["functions"][1]["refs"][0]["status"] = json!("dependent");
    document["unit"]["functions"][1]["refs"][0]["via"] = json!("def:interface");
    assert_eq!(call_arguments(document), vec![Argument::Opaque; 2]);
}

fn receiver(document: &mut Value) {
    callable(document)["receiver"] = json!("type:unit");
    let scan = &mut document["unit"]["functions"][0];
    scan["signature"]["receiver"] = json!({"symbol":"self","type":"type:unit"});
    let mut symbol = parameter("self");
    symbol["type"] = json!("type:unit");
    scan["symbols"].as_array_mut().unwrap().push(symbol);
}

#[test]
fn method_arguments_include_the_receiver_before_reordered_parameters() {
    let mut document = document();
    receiver(&mut document);
    let caller = &mut document["unit"]["functions"][1];
    node(caller, "caller.callee")["kind"] = json!("literal");
    node(caller, "caller.callee")["type"] = json!("type:unit");
    caller["refs"][0]["dispatch"] = json!("method");
    caller["refs"][0]["arguments"] = json!([
        {"position":0}, {"position":2,"value":"caller.zero"},
        {"position":1,"value":"caller.n"}
    ]);
    assert_eq!(proven_count(document), Bound::Input("n".into()));
}

/// `scan` as a static method, called through an instance whose receiver is written first.
fn through_an_instance(arguments: Value) -> Value {
    let mut document = document();
    let caller = &mut document["unit"]["functions"][1];
    node(caller, "caller.callee")["kind"] = json!("literal");
    caller["refs"][0]["dispatch"] = json!("method");
    caller["refs"][0]["arguments"] = arguments;
    document
}

#[test]
fn a_static_member_through_an_instance_binds_its_arguments_without_the_receiver() {
    let document = through_an_instance(json!([
        {}, {"position":1,"value":"caller.zero"}, {"position":0,"value":"caller.n"}
    ]));
    assert_eq!(proven_count(document), Bound::Input("n".into()));
}

#[test]
fn a_static_member_whose_receiver_binds_a_parameter_keeps_its_sizes_unavailable() {
    // Numbered as if the receiver were passed, which a static member isn't.
    assert_partial(through_an_instance(json!([
        {"position":0}, {"position":2,"value":"caller.zero"},
        {"position":1,"value":"caller.n"}
    ])));
}

/// The call made a call of a type, whose value is `scan`'s receiver.
fn constructor(document: &mut Value) {
    receiver(document);
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"def:Made","kind":"type","name":"Made","stable_identity":"Made","scope":"unit"
        }));
    document["unit"]["functions"][1]["refs"][1]["target"]["id"] = json!("def:Made");
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"type:Made","kind":"record","spelling":"Made","definition":"def:Made"
        }));
    node(&mut document["unit"]["functions"][1], "caller.callee")["type"] = json!("type:Made");
}

#[test]
fn a_constructor_binds_its_arguments_after_the_receiver_it_makes() {
    let mut document = document();
    constructor(&mut document);
    *bindings(&mut document) = json!([
        {"position":2,"parameter":"unused","value":"caller.zero"},
        {"position":1,"parameter":"limit","value":"caller.n"}
    ]);
    assert_eq!(proven_count(document), Bound::Input("n".into()));
}

#[test]
fn a_constructor_whose_argument_binds_the_receiver_keeps_its_sizes_unavailable() {
    // Numbered as if the call passed the receiver it makes, so `n` would bind `self`.
    let mut document = document();
    constructor(&mut document);
    assert_partial(document);
}

fn argument_call(document: &mut Value, index: usize) {
    let (id, name) = if index == 0 {
        ("caller.zero", "first")
    } else {
        ("caller.n", "second")
    };
    let function = &mut document["unit"]["functions"][1];
    let head = format!("{name}.head");
    let call = node(function, id);
    call["kind"] = json!("call");
    call["type"] = json!("type:u32");
    call["children"] = json!([head]);
    call.as_object_mut().unwrap().remove("value");
    let mut callee = expression(&head, "identifier", &[]);
    callee["type"] = json!(format!("type:{name}"));
    function["nodes"].as_array_mut().unwrap().push(callee);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != id);
    let definition = format!("def:{name}");
    let mut reference = reference(id, "definition", &definition);
    reference["dispatch"] = json!("static");
    function["refs"]
        .as_array_mut()
        .unwrap()
        .extend([reference, self::reference(&head, "definition", &definition)]);
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":definition,"kind":"function","name":name,"stable_identity":name,"scope":"unit",
            "compilation":"test","callable":{"parameters":[],"result":"type:u32","generics":0}
        }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":format!("type:{name}"),"kind":"function","spelling":name,"definition":definition,
            "parameters":[],"result":"type:u32"
        }));
}

#[test]
fn reordered_summary_slots_retain_every_call_in_written_evaluation_order() {
    let mut document = document();
    argument_call(&mut document, 0);
    argument_call(&mut document, 1);
    let programs = programs(document);
    let caller = &programs["test/caller"];
    let mut at = caller.entry;
    let mut calls = Vec::new();
    loop {
        let node = &caller.nodes[at as usize];
        if let Action::Call { callee, .. } = &node.action {
            calls.push(callee.as_str());
        }
        match &node.flow {
            Flow::Goto(next) => at = *next,
            Flow::Return => break,
            _ => panic!("unexpected branch: {node:?}"),
        }
    }
    assert_eq!(calls, ["first", "second", "scan"]);
    let outcome = runtime_bounds::analyze(caller);
    let Outcome::Proven(assessment) = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(assessment.calls.len(), 3);
    assert!(
        assessment
            .calls
            .iter()
            .all(|call| call.count == Bound::Const(1))
    );
}

#[test]
fn a_reordered_extent_uses_the_scanned_place_instead_of_the_shorter_unused_place() {
    let mut document = document();
    let mut scan = example()["unit"]["functions"][2].clone();
    scan["definition"] = json!("def:scan");
    scan["signature"]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "position":1,"symbol":"unused","type":"type:slice_ref"
        }));
    let mut unused = parameter("unused");
    unused["type"] = json!("type:slice_ref");
    scan["symbols"].as_array_mut().unwrap().push(unused);
    document["unit"]["functions"][0] = scan;
    callable(&mut document)["parameters"] = json!(["type:slice_ref", "type:slice_ref"]);
    let caller = &mut document["unit"]["functions"][1];
    caller["signature"]["parameters"][0]["type"] = json!("type:slice_ref");
    caller["signature"]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "position":1,"symbol":"short","type":"type:slice_ref"
        }));
    caller["symbols"][0]["type"] = json!("type:slice_ref");
    let mut short = parameter("short");
    short["type"] = json!("type:slice_ref");
    caller["symbols"].as_array_mut().unwrap().push(short);
    for id in ["caller.n", "caller.zero"] {
        let node = node(caller, id);
        node["kind"] = json!("identifier");
        node["type"] = json!("type:slice_ref");
        node.as_object_mut().unwrap().remove("value");
    }
    caller["refs"]
        .as_array_mut()
        .unwrap()
        .push(reference("caller.zero", "symbol", "short"));
    let bound = proven_count(document);
    assert_eq!(
        bound.evaluate(&|name| Some(if name.starts_with("len(n)") { 1000 } else { 1 })),
        Some(1000)
    );
}
