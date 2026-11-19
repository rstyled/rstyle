use super::{Binding, MAX_SCAN, MAX_SPINE, binding};
use crate::cpg::bounds::facts::Facts;
use crate::cpg::model::Document;
use crate::cpg::source::Sources;
use crate::cpg::tree::{Body, Tables};
use serde_json::{Value, json};

const SYMBOL: &str = "deferred:iterator";
const TYPE: &str = "deferred:state";
const DECLARATION: &str = "deferred:declaration";
const PATTERN: &str = "deferred:pattern";
const INIT: &str = "deferred:init";
const USE: &str = "deferred:use";
const TAIL: &str = "deferred:tail";
const METHOD: &str = "deferred:method";

fn expression(id: &str, kind: &str, children: &[&str]) -> Value {
    json!({"id":id,"kind":kind,"original_kind":kind,"children":children,
           "type":TYPE,"provenance":{"origin":"source"}})
}

fn function(document: &mut Value) -> &mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "top")
        .unwrap()
}

fn item<'a>(items: &'a mut Value, key: &str, value: &str) -> &'a mut Value {
    items
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item[key] == value)
        .unwrap()
}

fn node<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    item(&mut function(document)["nodes"], "id", id)
}

fn reference<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    item(&mut function(document)["refs"], "node", id)
}

fn symbol(document: &mut Value) -> &mut Value {
    item(&mut function(document)["symbols"], "id", SYMBOL)
}

fn method_reference(node: &str) -> Value {
    json!({"node":node,"status":"resolved","dispatch":"method",
           "target":{"kind":"definition","id":METHOD}})
}

fn document() -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../../testdata/example.cpg.json")).unwrap();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":TYPE,"kind":"opaque","spelling":"deferred state"
        }));
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":METHOD,"kind":"method","name":"method","stable_identity":"library::method",
            "scope":"library","standard":true
        }));
    let owner = function(&mut document);
    owner["root"] = json!("deferred:root");
    owner["nodes"] = json!([
        expression("deferred:root", "block", &[DECLARATION, TAIL]),
        expression(DECLARATION, "local_declaration", &[PATTERN, INIT]),
        expression(PATTERN, "pattern_binding", &[]),
        expression(INIT, "call", &["deferred:closure"]),
        expression("deferred:closure", "closure", &[]),
        expression(TAIL, "call", &[USE]),
        expression(USE, "identifier", &[]),
    ]);
    node(&mut document, "deferred:closure")["captures"] = json!([]);
    let owner = function(&mut document);
    owner["symbols"] = json!([{
        "id":SYMBOL,"name":"iterator","kind":"local","scope":"deferred",
        "mutable":false,"node":PATTERN,"type":TYPE
    }]);
    owner["refs"] = json!([
        method_reference(INIT), method_reference(TAIL),
        {"node":USE,"status":"resolved","target":{"kind":"symbol","id":SYMBOL}}
    ]);
    owner["accesses"] = json!([
        {"node":PATTERN,"symbol":SYMBOL,"mode":"write"},
        {"node":USE,"symbol":SYMBOL,"mode":"read"}
    ]);
    owner["places"] = json!([]);
    document
}

fn with_facts<R>(document: Value, check: impl FnOnce(&Facts<'_, '_>) -> R) -> R {
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
    let sources = Sources::new(unit).unwrap();
    check(&Facts {
        body: &body,
        tables: &tables,
        sources: &sources,
        pointer_bits: 64,
        destructors_are_calls: false,
    })
}

fn accepted(document: Value) -> bool {
    with_facts(document, |facts| {
        binding(
            facts,
            facts.body.node(DECLARATION).unwrap(),
            facts.body.node(TAIL).unwrap(),
        )
        .unwrap()
        .is_some()
    })
}

fn place(node: &str, access: &str) -> Value {
    json!({"id":"deferred:place","node":node,"root":{"symbol":SYMBOL},
           "path":[],"access":access,"mode":"value","escapes":false,"type":TYPE})
}

#[test]
fn lexical_binding_records_the_exact_initialization_and_use() {
    with_facts(document(), |facts| {
        let Binding {
            symbol,
            init,
            use_node,
        } = binding(
            facts,
            facts.body.node(DECLARATION).unwrap(),
            facts.body.node(TAIL).unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(symbol, SYMBOL);
        assert_eq!(init.id, INIT);
        assert_eq!(use_node.id, USE);
    });
}

#[test]
fn binding_patterns_must_be_simple_immutable_values_with_one_initializer() {
    for (pointer, value) in [
        ("/kind", json!("pattern_tuple")),
        ("/binding_mode", json!("shared")),
        ("/binding_mode", json!("exclusive")),
        ("/modifiers", json!(["mutable"])),
        ("/adjustments", json!([{"kind":"auto_deref","target":TYPE}])),
    ] {
        let mut changed = document();
        node(&mut changed, PATTERN)[pointer.trim_start_matches('/')] = value;
        assert!(!accepted(changed), "{pointer}");
    }
    for children in [json!([]), json!([PATTERN])] {
        let mut changed = document();
        node(&mut changed, DECLARATION)["children"] = children;
        assert!(!accepted(changed));
    }
    let mut non_declaration = document();
    node(&mut non_declaration, DECLARATION)["kind"] = json!("tuple");
    assert!(!accepted(non_declaration));
}

#[test]
fn symbol_identity_type_and_initialization_facts_must_agree() {
    for (field, value) in [
        ("mutable", json!(true)),
        ("kind", json!("parameter")),
        ("node", json!(null)),
        ("type", json!(null)),
        ("type", json!("type:u32")),
    ] {
        let mut changed = document();
        symbol(&mut changed)[field] = value;
        assert!(!accepted(changed), "{field}");
    }
    for id in [PATTERN, INIT, USE] {
        let mut changed = document();
        node(&mut changed, id)["type"] = json!("type:u32");
        assert!(!accepted(changed), "{id}");
    }
    let mut absent_type = document();
    absent_type["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .retain(|ty| ty["id"] != TYPE);
    assert!(!accepted(absent_type));
    let mut duplicate = document();
    let mut alias = symbol(&mut duplicate).clone();
    alias["id"] = json!("deferred:other");
    function(&mut duplicate)["symbols"]
        .as_array_mut()
        .unwrap()
        .push(alias);
    assert!(!accepted(duplicate));
}

#[test]
fn the_only_use_is_a_resolved_unadjusted_identifier() {
    for (field, value) in [
        ("status", json!("dependent")),
        ("status", json!("unresolved")),
        ("target", json!({"kind":"definition","id":SYMBOL})),
        ("target", json!({"kind":"symbol","id":"deferred:other"})),
        ("target", json!(null)),
    ] {
        let mut changed = document();
        reference(&mut changed, USE)[field] = value;
        assert!(!accepted(changed), "{field}");
    }
    for id in [INIT, USE] {
        let mut changed = document();
        node(&mut changed, id)["adjustments"] =
            json!([{"kind":"auto_ref","mode":"shared","target":TYPE}]);
        assert!(!accepted(changed), "{id}");
    }
    let mut absent = document();
    function(&mut absent)["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != USE);
    assert!(!accepted(absent));
}

#[test]
fn every_receiver_spine_call_requires_resolved_method_dispatch() {
    for (field, value) in [
        ("status", json!("dependent")),
        ("dispatch", json!(null)),
        ("dispatch", json!("static")),
        ("dispatch", json!("dynamic")),
        ("dispatch", json!("overloaded")),
        ("arguments", json!([])),
        ("target", json!({"kind":"symbol","id":METHOD})),
        ("target", json!({"kind":"definition"})),
        (
            "target",
            json!({"kind":"definition","id":"deferred:missing"}),
        ),
    ] {
        let mut changed = document();
        reference(&mut changed, TAIL)[field] = value;
        assert!(!accepted(changed), "{field}");
    }
    let mut empty = document();
    node(&mut empty, TAIL)["children"] = json!([]);
    assert!(!accepted(empty));
    let mut not_call = document();
    node(&mut not_call, TAIL)["kind"] = json!("reference");
    assert!(!accepted(not_call));
    let mut wrong_definition = document();
    item(&mut wrong_definition["unit"]["definitions"], "id", METHOD)["kind"] = json!("function");
    assert!(!accepted(wrong_definition));
}

#[test]
fn missing_duplicate_and_conflicting_access_rosters_are_rejected() {
    for removed in ["read", "write"] {
        let mut changed = document();
        function(&mut changed)["accesses"]
            .as_array_mut()
            .unwrap()
            .retain(|access| access["mode"] != removed);
        assert!(!accepted(changed), "{removed}");
    }
    for access in [
        json!({"node":USE,"symbol":SYMBOL,"mode":"read"}),
        json!({"node":PATTERN,"symbol":SYMBOL,"mode":"write"}),
        json!({"node":USE,"symbol":SYMBOL,"mode":"read_write"}),
        json!({"node":INIT,"symbol":SYMBOL,"mode":"write"}),
    ] {
        let mut changed = document();
        function(&mut changed)["accesses"]
            .as_array_mut()
            .unwrap()
            .push(access);
        assert!(!accepted(changed));
    }
}

#[test]
fn aliases_captures_and_references_outside_the_consuming_receiver_are_rejected() {
    let mut extra_reference = document();
    function(&mut extra_reference)["refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node":"deferred:closure","status":"resolved","target":{"kind":"symbol","id":SYMBOL}
        }));
    assert!(!accepted(extra_reference));
    let mut selected = document();
    reference(&mut selected, TAIL)["selected"] = json!({"kind":"symbol","id":SYMBOL});
    assert!(!accepted(selected));
    for mode in ["shared", "value", "exclusive", "unique"] {
        let mut captured = document();
        node(&mut captured, "deferred:closure")["captures"] = json!([{
            "symbol":SYMBOL,"mode":mode,"type":TYPE,"path":[]
        }]);
        assert!(!accepted(captured), "{mode}");
    }
    for kind in ["closure", "item_declaration"] {
        let mut unrecorded = document();
        let closure = node(&mut unrecorded, "deferred:closure");
        closure["kind"] = json!(kind);
        closure.as_object_mut().unwrap().remove("captures");
        assert!(!accepted(unrecorded), "{kind}");
    }
}

#[test]
fn only_matching_whole_value_places_with_explicit_non_escape_are_inert() {
    for (id, access) in [(USE, "read"), (PATTERN, "write")] {
        let mut valid = document();
        function(&mut valid)["places"] = json!([place(id, access)]);
        assert!(accepted(valid));
    }
    for (field, value) in [
        ("mode", json!("shared")),
        ("mode", json!("exclusive")),
        ("mode", json!("raw")),
        ("escapes", json!(null)),
        ("escapes", json!(true)),
        ("access", json!("read_write")),
        ("node", json!(INIT)),
        ("path", json!([{"deref":true}])),
        ("adjustments", json!([{"kind":"auto_deref","target":TYPE}])),
    ] {
        let mut changed = document();
        let mut exposure = place(USE, "read");
        exposure[field] = value;
        function(&mut changed)["places"] = json!([exposure]);
        assert!(!accepted(changed), "{field}");
    }
}

fn extend_spine(document: &mut Value, count: usize) {
    let mut receiver = USE.to_owned();
    for index in 0..count {
        let id = format!("deferred:stage-{index}");
        function(document)["nodes"]
            .as_array_mut()
            .unwrap()
            .push(expression(&id, "call", &[&receiver]));
        function(document)["refs"]
            .as_array_mut()
            .unwrap()
            .push(method_reference(&id));
        receiver = id;
    }
    node(document, TAIL)["children"] = json!([receiver]);
}

#[test]
fn receiver_spine_depth_is_bounded_and_detached_cycles_fail_closed() {
    let mut within = document();
    extend_spine(&mut within, MAX_SPINE - 1);
    assert!(accepted(within));
    let mut beyond = document();
    extend_spine(&mut beyond, MAX_SPINE);
    assert!(!accepted(beyond));
    let mut cycle = document();
    function(&mut cycle)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(expression("deferred:cycle", "call", &["deferred:cycle"]));
    function(&mut cycle)["refs"]
        .as_array_mut()
        .unwrap()
        .push(method_reference("deferred:cycle"));
    with_facts(cycle, |facts| {
        assert!(
            binding(
                facts,
                facts.body.node(DECLARATION).unwrap(),
                facts.body.node("deferred:cycle").unwrap()
            )
            .unwrap()
            .is_none()
        );
    });
}

#[test]
fn owner_rosters_and_nested_capture_work_have_bounded_scan_cost() {
    let mut accesses = document();
    function(&mut accesses)["accesses"] = json!(vec![
        json!({"node":USE,"symbol":SYMBOL,"mode":"read"});
        MAX_SCAN + 1
    ]);
    assert!(!accepted(accesses));
    let mut captures = document();
    node(&mut captures, "deferred:closure")["captures"] = json!(vec![
        json!({
            "symbol":"deferred:other","mode":"shared","type":TYPE
        });
        MAX_SCAN
    ]);
    assert!(!accepted(captures));
    let mut paths = document();
    node(&mut paths, "deferred:closure")["captures"] = json!([{
        "symbol":"deferred:other","mode":"shared","type":TYPE,
        "path":vec![json!({"kind":"field","type":TYPE,"member_index":0}); MAX_SCAN]
    }]);
    assert!(!accepted(paths));
}

#[test]
fn shared_dag_use_or_initializer_nodes_are_invalid_before_certification() {
    for id in [USE, INIT, PATTERN] {
        let mut changed = document();
        node(&mut changed, "deferred:root")["children"]
            .as_array_mut()
            .unwrap()
            .push(json!(id));
        let changed: Document = serde_json::from_value(changed).unwrap();
        let function = changed
            .unit
            .as_ref()
            .unwrap()
            .functions
            .as_ref()
            .unwrap()
            .iter()
            .find(|function| function.name == "top")
            .unwrap();
        assert!(Body::new(function).is_err(), "{id}");
    }
}
