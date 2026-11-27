//! Primitive flag and scratch-value writes must not hide an independent input-sized
//! scan. They still evaluate their values; indirect writes and operator effects stay unknown.

use super::bounds::report::from_records;
use super::records::Demand;
use super::run_records;
use serde_json::{Value, json};

const EXAMPLE: &str = include_str!("testdata/example.cpg.json");
const DEMAND: Demand = Demand {
    graphs: false,
    bounds: true,
    flow: false,
    values: false,
    defects: false,
};

struct Fixture(Value);

impl Fixture {
    fn new(kind: &str) -> Self {
        let mut document: Value = serde_json::from_str(EXAMPLE).unwrap();
        document["unit"]["functions"]
            .as_array_mut()
            .unwrap()
            .retain(|function| function["id"] == "fn:count");
        document["unit"]["types"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "type:write", "kind": kind, "spelling": kind}));
        let mut fixture = Self(document);
        fixture.add("write.pattern", "pattern_binding", &[]);
        fixture.add("write.value", "literal", &[]);
        fixture.add(
            "write.let",
            "local_declaration",
            &["write.pattern", "write.value"],
        );
        fixture.add("write.left", "identifier", &[]);
        fixture.add("write.right", "literal", &[]);
        fixture.add("write.assign", "assignment", &["write.left", "write.right"]);
        fixture.add("write.statement", "expression_statement", &["write.assign"]);
        fixture.function()["symbols"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": "write.local", "name": "scratch", "kind": "local",
                "scope": "count.block", "mutable": true,
                "type": "type:write", "node": "write.pattern"
            }));
        fixture.reference("write.left", "write.local");
        fixture.node("count.block")["children"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!("write.let"));
        fixture.node("count.then")["children"]
            .as_array_mut()
            .unwrap()
            .insert(0, json!("write.statement"));
        fixture
    }

    fn function(&mut self) -> &mut Value {
        &mut self.0["unit"]["functions"][0]
    }

    fn add(&mut self, id: &str, kind: &str, children: &[&str]) {
        let span = self.function()["span"].clone();
        self.function()["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": id, "kind": kind, "original_kind": kind,
                "children": children, "span": span, "type": "type:write",
                "provenance": {"origin": "source"}
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

    fn zero(&mut self) {
        self.add("write.zero", "literal", &[]);
        self.node("write.zero")["type"] = json!("type:u32");
        self.node("write.zero")["value"] = json!({"kind": "integer", "value": "0"});
    }

    fn row(self) -> Value {
        let records = run_records(vec![self.0], DEMAND).unwrap();
        let report = serde_json::to_value(from_records(&records, None)).unwrap();
        report["rows"][0].clone()
    }
}

fn input_scan(row: &Value) {
    assert_eq!(
        row["result"]["Proven"]["loops"].as_array().map(Vec::len),
        Some(1),
        "{row}"
    );
    assert_eq!(
        row["result"]["Proven"]["loop_body_entries"],
        json!({"Input": "len(items)#0"}),
        "{row}"
    );
}

fn values_written(program: &Value, name: &str) -> Vec<Value> {
    let variable = program["program"]["variables"]
        .as_array()
        .unwrap()
        .iter()
        .position(|variable| variable["name"] == name)
        .unwrap();
    program["program"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["action"]["Assign"]["variable"] == json!(variable))
        .map(|node| node["action"]["Assign"]["value"].clone())
        .collect()
}

#[test]
fn writes_to_primitive_scratch_values_preserve_the_input_scan() {
    for kind in ["bool", "char", "float", "unit", "string"] {
        input_scan(&Fixture::new(kind).row());
    }
}

#[test]
fn builtin_compound_writes_to_primitive_scratch_values_preserve_the_scan() {
    let mut fixture = Fixture::new("bool");
    fixture.node("write.assign")["kind"] = json!("compound_assignment");
    fixture.node("write.assign")["operator"] = json!("or");
    input_scan(&fixture.row());
}

#[test]
fn a_scratch_assignment_still_counts_the_call_computing_its_value() {
    let mut fixture = Fixture::new("bool");
    fixture.add("write.callee", "identifier", &[]);
    fixture.node("write.right")["kind"] = json!("call");
    fixture.node("write.right")["children"] = json!(["write.callee"]);
    let row = fixture.row();
    input_scan(&row);
    let calls = row["result"]["Proven"]["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 1, "{row}");
    assert_eq!(calls[0]["target"], json!("Unresolved"), "{row}");
    assert!(row["composed"].get("Partial").is_some(), "{row}");
}

#[test]
fn a_scratch_assignment_still_lowers_counter_writes_in_its_value() {
    let mut fixture = Fixture::new("bool");
    fixture.add("write.counter", "identifier", &[]);
    fixture.node("write.counter")["type"] = json!("type:u32");
    fixture.reference("write.counter", "count.total");
    fixture.zero();
    fixture.add(
        "write.reset",
        "assignment",
        &["write.counter", "write.zero"],
    );
    fixture.add(
        "write.reset_statement",
        "expression_statement",
        &["write.reset"],
    );
    fixture.node("write.right")["kind"] = json!("block");
    fixture.add("write.false", "literal", &[]);
    fixture.node("write.right")["children"] = json!(["write.reset_statement", "write.false"]);
    let records = run_records(vec![fixture.0], DEMAND).unwrap();
    let program = serde_json::to_value(records[0].bounds.as_ref().unwrap()).unwrap();
    assert_eq!(
        values_written(&program, "total"),
        [json!({"Const": 0}), json!({"Const": 0})],
        "{program}"
    );
}

#[test]
fn a_scratch_assignment_keeps_cleanup_of_values_computed_on_its_right() {
    let mut fixture = Fixture::new("bool");
    fixture.0["coverage"]["capabilities"]["implicit-calls"]["status"] = json!("approximated");
    fixture.0["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "type:guard", "kind": "record", "spelling": "Guard", "definition": "def:guard"
        }));
    fixture.0["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "def:guard", "kind": "type", "name": "Guard", "stable_identity": "Guard",
            "scope": "unit", "destructor": true
        }));
    fixture.add("write.guard", "record_construction", &[]);
    fixture.node("write.guard")["type"] = json!("type:guard");
    fixture.add(
        "write.guard_statement",
        "expression_statement",
        &["write.guard"],
    );
    fixture.add("write.false", "literal", &[]);
    fixture.node("write.right")["kind"] = json!("block");
    fixture.node("write.right")["children"] = json!(["write.guard_statement", "write.false"]);
    let row = fixture.row();
    assert!(row["result"].get("Unknown").is_some(), "{row}");
    assert!(
        row["result"]["Unknown"]["reason"]
            .as_str()
            .unwrap()
            .contains("drop glue"),
        "{row}"
    );
}

#[test]
fn primitive_writes_with_operator_or_place_effects_stay_unknown() {
    let cases = [
        "overloaded",
        "field_access",
        "index_access",
        "deref",
        "adjusted",
    ];
    for case in cases {
        let mut fixture = Fixture::new("bool");
        match case {
            "overloaded" => fixture.function()["refs"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "node": "write.assign", "status": "unresolved", "dispatch": "overloaded",
                    "reason": "an operator implementation computes the write"
                })),
            "adjusted" => {
                fixture.node("write.left")["adjustments"] = json!([{
                    "kind": "auto_deref", "target": "type:write"
                }])
            }
            kind => {
                fixture.add("write.place_operand", "identifier", &[]);
                fixture.reference("write.place_operand", "write.local");
                fixture.node("write.left")["kind"] =
                    json!(if kind == "deref" { "operator" } else { kind });
                fixture.node("write.left")["operator"] = json!("deref");
                fixture.node("write.left")["children"] = json!(["write.place_operand"]);
                if kind == "index_access" {
                    fixture.zero();
                    fixture.node("write.left")["children"] =
                        json!(["write.place_operand", "write.zero"]);
                }
            }
        }
        assert!(fixture.row()["result"].get("Unknown").is_some(), "{case}");
    }
}

#[test]
fn aggregate_reference_and_missing_type_writes_stay_unknown() {
    for kind in [
        "sequence",
        "record",
        "reference",
        "pointer",
        "opaque",
        "integer",
    ] {
        assert!(
            Fixture::new(kind).row()["result"].get("Unknown").is_some(),
            "{kind}"
        );
    }
    let mut fixture = Fixture::new("bool");
    fixture
        .node("write.left")
        .as_object_mut()
        .unwrap()
        .remove("type");
    assert!(fixture.row()["result"].get("Unknown").is_some());
}

#[test]
fn replacing_a_selected_extent_stays_unknown() {
    let mut fixture = Fixture::new("bool");
    fixture.node("write.left")["type"] = json!("type:slice_ref");
    let reference = fixture.function()["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "write.left")
        .unwrap();
    reference["target"]["id"] = json!("count.items");
    assert!(fixture.row()["result"].get("Unknown").is_some());
}

#[test]
fn primitive_destructuring_keeps_scalar_havoc_and_the_input_scan() {
    let mut fixture = Fixture::new("bool");
    fixture.add("write.counter", "identifier", &[]);
    fixture.node("write.counter")["type"] = json!("type:u32");
    fixture.reference("write.counter", "count.total");
    fixture.zero();
    fixture.add("write.target", "tuple", &["write.left", "write.counter"]);
    fixture.add("write.values", "tuple", &["write.right", "write.zero"]);
    fixture.node("write.assign")["children"] = json!(["write.target", "write.values"]);
    let records = run_records(vec![fixture.0], DEMAND).unwrap();
    let program = serde_json::to_value(records[0].bounds.as_ref().unwrap()).unwrap();
    assert!(
        values_written(&program, "total").contains(&json!("Unknown")),
        "{program}"
    );
    let report = serde_json::to_value(from_records(&records, None)).unwrap();
    input_scan(&report["rows"][0]);
}

#[test]
fn ignoring_boolean_storage_does_not_prove_a_boolean_controlled_loop() {
    let mut fixture = Fixture::new("bool");
    fixture.add("write.condition", "identifier", &[]);
    fixture.reference("write.condition", "write.local");
    fixture.add("write.body", "block", &["write.statement"]);
    fixture.add("write.while", "while", &["write.condition", "write.body"]);
    fixture.add(
        "write.while_statement",
        "expression_statement",
        &["write.while"],
    );
    fixture.node("count.then")["children"] = json!(["write.while_statement", "count.stmt_add"]);
    let row = fixture.row();
    assert!(row["result"].get("Partial").is_some(), "{row}");
}

#[test]
fn overloaded_destructuring_stays_unknown() {
    let mut fixture = Fixture::new("bool");
    fixture.add("write.target", "tuple", &["write.left"]);
    fixture.node("write.assign")["children"] = json!(["write.target", "write.right"]);
    fixture.function()["refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node": "write.assign", "status": "unresolved", "dispatch": "overloaded",
            "reason": "an operator implementation computes the write"
        }));
    assert!(fixture.row()["result"].get("Unknown").is_some());
}
