//! A scan can write unrelated value storage without acquiring an unknown bound.
//! Stored shapes must be complete; indirect-reference types, callbacks and hidden
//! cleanup cannot be certified as irrelevant storage.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::cpg::bounds::report::from_records;
use rstyle_core::cpg::records::Demand;
use rstyle_core::runtime_bounds::{self, Action, Bound, Outcome, Program};
use serde_json::{Value, json};

struct Store(Value);

impl Store {
    fn new(shape: Value) -> Self {
        let mut document: Value =
            serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
        document["unit"]["functions"]
            .as_array_mut()
            .unwrap()
            .retain(|function| function["id"] == "fn:count");
        let mut store = Self(document);
        store.add_type(shape);
        store.add_type(json!({"id": "type:text", "kind": "string", "spelling": "text"}));
        store.0["unit"]["definitions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": "def:stored", "kind": "type", "name": "Stored", "stable_identity": "Stored",
                "scope": "unit", "destructor": false
            }));
        store.install_local();
        store.install_assignment();
        store
    }

    fn function(&mut self) -> &mut Value {
        &mut self.0["unit"]["functions"][0]
    }

    fn add_type(&mut self, ty: Value) {
        self.0["unit"]["types"].as_array_mut().unwrap().push(ty);
    }

    fn add(&mut self, id: &str, kind: &str, children: &[&str]) {
        let span = self.function()["span"].clone();
        self.function()["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": id, "kind": kind, "original_kind": kind, "children": children,
                "span": span, "type": "type:stored", "provenance": {"origin": "source"}
            }));
    }

    fn node(&mut self, id: &str) -> &mut Value {
        self.function()["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["id"] == id)
            .unwrap()
    }

    fn reference(&mut self, node: &str, symbol: &str) {
        self.function()["refs"].as_array_mut().unwrap().push(json!({
            "node": node, "status": "resolved", "target": {"kind": "symbol", "id": symbol}
        }));
    }

    fn install_local(&mut self) {
        self.add("store.pattern", "pattern_binding", &[]);
        self.add("store.initial", "literal", &[]);
        self.add(
            "store.local",
            "local_declaration",
            &["store.pattern", "store.initial"],
        );
        self.function()["symbols"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": "store.symbol", "name": "stored", "kind": "local", "scope": "count.block",
                "mutable": true, "node": "store.pattern", "type": "type:stored"
            }));
        self.node("count.block")["children"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!("store.local"));
    }

    fn install_assignment(&mut self) {
        self.add("store.left", "identifier", &[]);
        self.reference("store.left", "store.symbol");
        self.add("store.right", "literal", &[]);
        self.add("store.assign", "assignment", &["store.left", "store.right"]);
        self.node("store.assign")["type"] = json!("type:unit");
        self.add("store.statement", "expression_statement", &["store.assign"]);
        self.node("count.then")["children"]
            .as_array_mut()
            .unwrap()
            .insert(0, json!("store.statement"));
    }

    fn program(self) -> Program {
        let mut programs = rstyle_core::cpg::run_bounds(vec![self.0]).unwrap();
        let (_, Input::Captured { program }) = programs.remove(0) else {
            panic!("expected captured bounds");
        };
        program
    }

    fn bound(self) -> Bound {
        let outcome = runtime_bounds::analyze(&self.program());
        let Outcome::Proven(assessment) = outcome else {
            panic!("expected a proven scan: {outcome:?}");
        };
        assert_eq!(assessment.loops.len(), 1);
        assessment.loop_body_entries
    }

    fn unknown(self) {
        let outcome = runtime_bounds::analyze(&self.program());
        assert!(matches!(outcome, Outcome::Unknown { .. }), "{outcome:?}");
    }
}

fn shape(kind: &str) -> Value {
    json!({"id": "type:stored", "kind": kind, "spelling": "Stored"})
}

fn sequence(element: &str) -> Value {
    let mut ty = shape("sequence");
    ty["element"] = json!(element);
    ty["fixed"] = json!(false);
    ty
}

fn record(fields: Value) -> Value {
    let mut ty = shape("record");
    ty["definition"] = json!("def:stored");
    ty["fields"] = fields;
    ty
}

fn sum(variants: Value) -> Value {
    let mut ty = shape("sum");
    ty["definition"] = json!("def:stored");
    ty["variants"] = variants;
    ty
}

#[test]
fn strings_and_closed_value_shapes_preserve_an_input_sized_scan() {
    let mut tuple = shape("tuple");
    tuple["items"] = json!(["type:text", "type:u32", "type:bool"]);
    let shapes = [
        shape("string"),
        tuple,
        sequence("type:text"),
        record(json!([{"name": "label", "type": "type:text"}])),
        sum(json!([
            {"name": "Empty", "shape": "unit"},
            {"name": "Label", "shape": "tuple", "fields": [{"name": "0", "type": "type:text"}]}
        ])),
    ];
    for ty in shapes {
        assert_eq!(Store::new(ty).bound(), Bound::Input("len(items)#0".into()));
    }
}

#[test]
fn a_sequence_of_explicit_value_records_is_safe_scratch_storage() {
    let mut store = Store::new(sequence("type:element"));
    store.add_type(json!({
        "id": "type:element", "kind": "record", "spelling": "Element", "definition": "def:stored",
        "fields": [{"name": "label", "type": "type:text"}, {"name": "count", "type": "type:u32"}]
    }));
    assert_eq!(store.bound(), Bound::Input("len(items)#0".into()));
}

#[test]
fn a_store_still_evaluates_a_call_that_builds_the_value() {
    let mut store = Store::new(sequence("type:text"));
    store.add("store.callee", "identifier", &[]);
    store.node("store.right")["kind"] = json!("call");
    store.node("store.right")["children"] = json!(["store.callee"]);
    let demand = Demand {
        bounds: true,
        graphs: false,
        flow: false,
        defects: false,
    };
    let records = rstyle_core::cpg::run_records(vec![store.0], demand).unwrap();
    let report = serde_json::to_value(from_records(&records, None)).unwrap();
    let row = &report["rows"][0];
    assert_eq!(
        row["result"]["Proven"]["loops"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        row["result"]["Proven"]["calls"].as_array().unwrap().len(),
        1
    );
    assert!(row["composed"].get("Partial").is_some(), "{row}");
}

#[test]
fn incomplete_value_shapes_do_not_default_to_safe_storage() {
    let mut record_without_fields = shape("record");
    record_without_fields["definition"] = json!("def:stored");
    let mut sum_without_variants = shape("sum");
    sum_without_variants["definition"] = json!("def:stored");
    for ty in [
        shape("tuple"),
        shape("sequence"),
        record_without_fields,
        sum_without_variants,
        shape("record"),
        shape("sum"),
        sum(json!([{"name": "Unknown"}])),
    ] {
        Store::new(ty).unknown();
    }
}

#[test]
fn unknown_nominal_definitions_and_missing_child_types_stay_unknown() {
    let mut missing_nominal = record(json!([]));
    missing_nominal["definition"] = json!("def:missing");
    for ty in [missing_nominal, sequence("type:missing")] {
        Store::new(ty).unknown();
    }
}

#[test]
fn a_nested_destructor_cannot_hide_behind_a_unit_typed_assignment() {
    let mut store = Store::new(record(json!([{"name": "inner", "type": "type:dropping"}])));
    store.add_type(json!({
        "id": "type:dropping", "kind": "record", "spelling": "Dropping",
        "definition": "def:dropping", "fields": []
    }));
    store.0["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "def:dropping", "kind": "type", "name": "Dropping", "stable_identity": "Dropping",
            "scope": "unit", "destructor": true
        }));
    // Loss of the value's type must not erase cleanup of the old, typed storage.
    store.node("store.local")["children"] = json!(["store.pattern"]);
    store
        .node("store.right")
        .as_object_mut()
        .unwrap()
        .remove("type");
    store.unknown();
}

#[test]
fn a_direct_nominal_destructor_keeps_the_store_unknown() {
    let mut store = Store::new(record(json!([])));
    let definition = store.0["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "def:stored")
        .unwrap();
    definition["destructor"] = json!(true);
    store.unknown();
}

#[test]
fn callable_reference_and_unknown_members_are_not_value_only() {
    for kind in [
        "reference",
        "pointer",
        "function",
        "closure",
        "interface",
        "parameter",
        "opaque",
    ] {
        let mut store = Store::new(sequence("type:element"));
        let mut element = shape(kind);
        element["id"] = json!("type:element");
        store.add_type(element);
        store.unknown();
    }
}

#[test]
fn recursive_and_excessively_wide_shapes_exhaust_certification_safely() {
    let recursive = sequence("type:stored");
    Store::new(recursive).unknown();
    let mut wide = shape("tuple");
    wide["items"] = json!(vec!["type:text"; 129]);
    Store::new(wide).unknown();
}

#[test]
fn indirect_writes_and_overloaded_assignments_stay_unknown() {
    for kind in ["field_access", "operator", "index_access", "overloaded"] {
        let mut store = Store::new(shape("string"));
        if kind == "overloaded" {
            store.function()["refs"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "node": "store.assign", "status": "unresolved", "dispatch": "overloaded",
                    "reason": "a user implementation writes the value"
                }));
        } else {
            store.add("store.base", "identifier", &[]);
            store.reference("store.base", "store.symbol");
            store.node("store.left")["kind"] = json!(kind);
            store.node("store.left")["operator"] = json!("deref");
            store.node("store.left")["children"] = json!(["store.base"]);
            if kind == "index_access" {
                store.add("store.index", "literal", &[]);
                store.node("store.left")["children"] = json!(["store.base", "store.index"]);
            }
        }
        store.unknown();
    }
}

#[test]
fn replacing_the_sequence_that_supplies_the_scan_extent_stays_unknown() {
    let mut store = Store::new(shape("string"));
    store.node("store.left")["type"] = json!("type:u32_slice");
    store.node("count.items")["type"] = json!("type:u32_slice");
    store.function()["signature"]["parameters"][0]["type"] = json!("type:u32_slice");
    store.function()["symbols"][0]["type"] = json!("type:u32_slice");
    let reference = store.function()["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "store.left")
        .unwrap();
    reference["target"]["id"] = json!("count.items");
    store.unknown();
}

#[test]
fn scalar_counter_assignments_are_still_recorded() {
    let mut store = Store::new(shape("string"));
    store.node("store.left")["type"] = json!("type:u32");
    let reference = store.function()["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "store.left")
        .unwrap();
    reference["target"]["id"] = json!("count.total");
    let program = store.program();
    let variable = program
        .variables
        .iter()
        .position(|variable| variable.name == "total")
        .unwrap() as u32;
    assert!(program.nodes.iter().any(|node| matches!(
        node.action, Action::Assign { variable: at, value: runtime_bounds::Value::Unknown } if at == variable
    )));
}

#[test]
fn nominal_strings_and_sequences_require_safe_destructor_facts() {
    for mut ty in [shape("string"), sequence("type:text")] {
        ty["definition"] = json!("def:stored");
        let mut store = Store::new(ty);
        let definition = store.0["unit"]["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|definition| definition["id"] == "def:stored")
            .unwrap();
        definition["destructor"] = json!(true);
        store.unknown();
    }
}

#[test]
fn generic_arguments_cannot_hide_a_callable() {
    for kind in ["string", "sequence", "record", "sum"] {
        let mut ty = match kind {
            "sequence" => sequence("type:text"),
            "record" => record(json!([])),
            "sum" => sum(json!([])),
            _ => shape(kind),
        };
        ty["arguments"] = json!(["type:argument"]);
        let mut store = Store::new(ty);
        store.add_type(json!({"id": "type:argument", "kind": "function", "spelling": "callback"}));
        store.unknown();
    }
}
