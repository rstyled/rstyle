//! The rules over functions written in the profile's terms, whatever wrote them.

use super::super::layout::bundle;
use super::super::source::Sources;
use super::super::tree::Tables;
use super::*;
use serde_json::{Value, json};

const UNIT_TYPE: &str = "type:unit";
const INT: &str = "type:int";
const OPAQUE: &str = "type:opaque";
const GUARD: &str = "type:guard";
const PARAMETER: &str = "type:parameter";
const GENERIC: &str = "type:generic";
const FILE: &str = "file:a";

/// A statement of a straight-line body, one line each, on the line after the last.
enum Line {
    /// `name := f()`
    Declare(&'static str),
    /// `name := 0`
    DeclareLiteral(&'static str),
    /// `var name int`
    DeclareEmpty(&'static str),
    /// `name = f()`
    Assign(&'static str),
    /// `_ = name`
    Read(&'static str),
}

/// A function of those lines, and what makes its parts any different.
#[derive(Default)]
struct Body {
    nodes: Vec<Value>,
    edges: Vec<Value>,
    last: String,
    accesses: Vec<Value>,
    symbols: Vec<Value>,
    statements: Vec<String>,
}

fn span(line: usize) -> Value {
    let start = line.saturating_sub(1) * 10;
    json!({"file": FILE, "start": start, "end": start + 5})
}

impl Body {
    fn new() -> Self {
        Self {
            last: "entry".into(),
            ..Self::default()
        }
    }

    /// Adds a node on the path, which runs after the nodes before it.
    fn node(&mut self, id: &str, (kind, line): (&str, usize), children: &[&str]) {
        self.nodes.push(json!({
            "id": id, "kind": kind, "original_kind": kind, "children": children, "type": UNIT_TYPE,
            "span": span(line), "provenance": {"origin": "source"},
        }));
        self.edges.push(
            json!({"id": format!("e{}", self.edges.len()), "source": self.last,
                               "target": id, "kind": "normal"}),
        );
        self.last = id.into();
    }

    fn symbol(&mut self, name: &str, declared_at: &str) {
        if !self.symbols.iter().any(|symbol| symbol["name"] == name) {
            self.symbols.push(
                json!({"id": format!("s:{name}"), "name": name, "kind": "local",
                                     "scope": "f", "node": declared_at, "mutable": true,
                                     "type": INT}),
            );
        }
    }

    fn access(&mut self, node: &str, name: &str, mode: &str) {
        self.accesses
            .push(json!({"node": node, "symbol": format!("s:{name}"), "mode": mode}));
    }

    fn line(&mut self, number: usize, line: &Line) {
        let statement = format!("statement{number}");
        let (value, pattern, target) = (
            format!("value{number}"),
            format!("pattern{number}"),
            format!("target{number}"),
        );
        match *line {
            Line::Declare(name) | Line::DeclareLiteral(name) => {
                let kind = match line {
                    Line::Declare(_) => "call",
                    _ => "literal",
                };
                self.node(&value, (kind, number), &[]);
                self.node(&pattern, ("pattern_binding", number), &[]);
                self.symbol(name, &pattern);
                self.access(&pattern, name, "write");
                self.node(
                    &statement,
                    ("local_declaration", number),
                    &[&pattern, &value],
                );
            }
            Line::DeclareEmpty(name) => {
                self.node(&pattern, ("pattern_binding", number), &[]);
                self.symbol(name, &pattern);
                self.access(&pattern, name, "write");
                self.node(&statement, ("local_declaration", number), &[&pattern]);
            }
            Line::Assign(name) => {
                self.node(&value, ("call", number), &[]);
                self.node(&target, ("identifier", number), &[]);
                self.access(&target, name, "write");
                self.node(&statement, ("assignment", number), &[&target, &value]);
            }
            Line::Read(name) => {
                self.node(&target, ("identifier", number), &[]);
                self.access(&target, name, "read");
                self.node(&statement, ("expression_statement", number), &[&target]);
            }
        }
        self.statements.push(statement);
    }

    fn document(mut self) -> Value {
        self.edges
            .push(json!({"id": "out", "source": self.last, "target": "exit", "kind": "normal"}));
        self.nodes.push(json!({
            "id": "root", "kind": "block", "original_kind": "block", "children": self.statements,
            "type": UNIT_TYPE, "span": span(0), "provenance": {"origin": "source"},
        }));
        let function = json!({
            "id": "fn:f", "definition": "def:f", "name": "f", "span": span(0),
            "signature": {"span": span(0), "parameters": [], "result": UNIT_TYPE, "generics": [],
                          "lifetimes": [], "outlives": [], "modifiers": []},
            "root": "root", "nodes": self.nodes,
            "cfg": {"entry": "entry", "exits": [{"id": "exit", "kind": "normal"}], "edges": self.edges},
            "symbols": self.symbols, "accesses": self.accesses, "refs": [], "places": [], "values": [],
        });
        let mut document: Value =
            serde_json::from_str(include_str!("../testdata/example.cpg.json")).unwrap();
        let unit = &mut document["unit"];
        unit["functions"] = json!([function]);
        unit["definitions"] = json!([
            {"id": "def:f", "kind": "function", "name": "f", "stable_identity": "f", "scope": "unit"},
            {"id": "def:guard", "kind": "type", "name": "Guard", "stable_identity": "Guard",
             "scope": "unit", "destructor": true},
        ]);
        unit["types"] = json!([
            {"id": UNIT_TYPE, "kind": "unit", "spelling": "()"},
            {"id": INT, "kind": "integer", "spelling": "int", "bits": 64, "signed": true},
            {"id": OPAQUE, "kind": "opaque", "spelling": "volatile int"},
            {"id": GUARD, "kind": "record", "spelling": "Guard", "definition": "def:guard"},
            {"id": PARAMETER, "kind": "parameter", "spelling": "T", "index": 0},
            {"id": GENERIC, "kind": "record", "spelling": "Box<T>", "arguments": [PARAMETER]},
        ]);
        unit["files"] = json!([{"id": FILE, "path": "a.go", "digest": "", "origin": "source",
                                 "content": lines_of_text()}]);
        document
    }
}

/// Ten bytes a line, so a span's offset says its line.
fn lines_of_text() -> String {
    "ok .....\n ".repeat(40)
}

fn document_of(lines: &[Line]) -> Value {
    let mut body = Body::new();
    for (index, line) in lines.iter().enumerate() {
        body.line(index + 1, line);
    }
    body.document()
}

fn examined(document: Value) -> Examined {
    let document = bundle(vec![document]).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let function = &unit.functions.as_ref().unwrap()[0];
    let tables = Tables::new(unit).unwrap();
    let sources = Sources::new(unit).unwrap();
    examine((&document.coverage, "test"), function, &tables, &sources)
}

/// The lines a value is reported stored on, by binding.
fn dead_stores(document: Value) -> Vec<(String, usize)> {
    let Examined::Read { defects, .. } = examined(document) else {
        panic!("the function was not read");
    };
    defects
        .into_iter()
        .map(|defect| {
            assert_eq!(defect.rule, "dead_store");
            let name = defect.message.split('`').nth(1).unwrap().to_owned();
            (name, defect.line)
        })
        .collect()
}

fn found(lines: &[Line]) -> Vec<(String, usize)> {
    dead_stores(document_of(lines))
}

#[test]
fn a_value_replaced_before_any_read_is_dead_and_the_one_read_is_not() {
    use Line::*;
    assert_eq!(
        found(&[Declare("err"), Assign("err"), Read("err")]),
        [("err".to_owned(), 1)]
    );
}

#[test]
fn a_store_nothing_after_it_reads_is_dead() {
    use Line::*;
    assert_eq!(
        found(&[Declare("v"), Read("v"), Assign("v")]),
        [("v".to_owned(), 3)]
    );
}

#[test]
fn a_value_read_before_it_is_replaced_is_alive() {
    use Line::*;
    assert!(found(&[Declare("v"), Read("v"), Assign("v"), Read("v")]).is_empty());
}

#[test]
fn a_starting_value_and_an_empty_declaration_drop_nothing_computed() {
    use Line::*;
    assert!(found(&[DeclareLiteral("n"), Assign("n"), Read("n")]).is_empty());
    assert!(found(&[DeclareEmpty("n"), Assign("n"), Read("n")]).is_empty());
}

#[test]
fn assigning_a_literal_still_stores_a_value() {
    use Line::*;
    let mut document = document_of(&[Declare("v"), Read("v"), Assign("v")]);
    let nodes = document["unit"]["functions"][0]["nodes"]
        .as_array_mut()
        .unwrap();
    let value = nodes
        .iter_mut()
        .find(|node| node["id"] == "value3")
        .unwrap();
    value["kind"] = json!("literal");
    assert_eq!(dead_stores(document), [("v".to_owned(), 3)]);
}

/// A document where everything but one change would report `v` on line 1.
fn dead_v() -> Value {
    use Line::*;
    document_of(&[Declare("v"), Assign("v"), Read("v")])
}

fn with_symbol_v(document: &mut Value, change: impl Fn(&mut Value)) {
    let symbols = document["unit"]["functions"][0]["symbols"]
        .as_array_mut()
        .unwrap();
    change(
        symbols
            .iter_mut()
            .find(|symbol| symbol["name"] == "v")
            .unwrap(),
    );
}

#[test]
fn the_fixture_reports_what_the_others_change() {
    assert_eq!(dead_stores(dead_v()), [("v".to_owned(), 1)]);
}

#[test]
fn a_leading_underscore_says_the_store_is_deliberate() {
    use Line::*;
    assert!(found(&[Declare("_v"), Assign("_v"), Read("_v")]).is_empty());
}

#[test]
fn a_destructor_is_what_a_binding_can_be_for() {
    let mut document = dead_v();
    with_symbol_v(&mut document, |symbol| symbol["type"] = json!(GUARD));
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_destructor_in_a_type_argument_counts_too() {
    let mut document = dead_v();
    document["unit"]["types"].as_array_mut().unwrap().push(
        json!({"id": "type:box", "kind": "record", "spelling": "Box<Guard>",
                     "arguments": [GUARD]}),
    );
    with_symbol_v(&mut document, |symbol| symbol["type"] = json!("type:box"));
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_type_the_frontend_cant_describe_might_do_anything_when_written() {
    let mut document = dead_v();
    with_symbol_v(&mut document, |symbol| symbol["type"] = json!(OPAQUE));
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_type_parameter_can_be_anything_when_written() {
    for ty in [PARAMETER, GENERIC] {
        let mut document = dead_v();
        with_symbol_v(&mut document, |symbol| symbol["type"] = json!(ty));
        assert!(dead_stores(document).is_empty(), "{ty}");
    }
}

#[test]
fn a_read_off_the_flow_cant_be_placed_so_nothing_is_said_of_its_binding() {
    let mut document = dead_v();
    let function = &mut document["unit"]["functions"][0];
    function["accesses"]
        .as_array_mut()
        .unwrap()
        .push(json!({"node": "root", "symbol": "s:v", "mode": "read"}));
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_binding_with_no_type_recorded_is_left_alone() {
    let mut document = dead_v();
    with_symbol_v(&mut document, |symbol| {
        symbol.as_object_mut().unwrap().remove("type");
    });
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_named_result_is_read_by_the_return() {
    let mut document = dead_v();
    with_symbol_v(&mut document, |symbol| {
        symbol.as_object_mut().unwrap().remove("node");
    });
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_parameter_replaced_before_any_read_is_dead() {
    use Line::*;
    let mut document = document_of(&[Assign("a")]);
    let function = &mut document["unit"]["functions"][0];
    function["symbols"] = json!([{"id": "s:a", "name": "a", "kind": "parameter", "scope": "f",
                                  "mutable": true, "type": INT}]);
    function["signature"]["parameters"] = json!([{"position": 0, "symbol": "s:a", "type": INT}]);
    assert_eq!(dead_stores(document), [("a".to_owned(), 1)]);
}

#[test]
fn what_a_closure_captures_it_can_read_whenever_it_runs() {
    let mut document = dead_v();
    let nodes = document["unit"]["functions"][0]["nodes"]
        .as_array_mut()
        .unwrap();
    let closure = nodes.iter_mut().find(|node| node["id"] == "root").unwrap();
    closure["captures"] = json!([{"symbol": "s:v", "mode": "shared", "type": INT}]);
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_raw_pointer_to_a_binding_can_read_it_later() {
    let mut document = dead_v();
    document["unit"]["functions"][0]["places"] = json!([
        {"id": "place", "node": "pattern1", "root": {"symbol": "s:v"}, "path": [],
         "access": "read_write", "mode": "raw", "type": INT}
    ]);
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_store_a_macro_wrote_is_not_the_authors() {
    let mut document = dead_v();
    let nodes = document["unit"]["functions"][0]["nodes"]
        .as_array_mut()
        .unwrap();
    let pattern = nodes
        .iter_mut()
        .find(|node| node["id"] == "pattern1")
        .unwrap();
    pattern["provenance"] = json!({"origin": "macro_expansion", "call_site": span(1)});
    assert!(dead_stores(document).is_empty());
}

#[test]
fn a_binding_a_loop_or_match_makes_is_not_a_store() {
    let mut document = dead_v();
    let nodes = document["unit"]["functions"][0]["nodes"]
        .as_array_mut()
        .unwrap();
    let declaration = nodes
        .iter_mut()
        .find(|node| node["id"] == "statement1")
        .unwrap();
    declaration["kind"] = json!("match_arm");
    assert!(dead_stores(document).is_empty());
}

#[test]
fn every_part_of_a_tuple_target_is_a_store() {
    use Line::*;
    let mut document = document_of(&[Declare("x"), Assign("y"), Read("y")]);
    let function = &mut document["unit"]["functions"][0];
    let nodes = function["nodes"].as_array_mut().unwrap();
    // `x, y := f()` then `y = f()`: the pattern holds both, and `x` is never read either.
    let pattern_parts = json!({"id": "tuple1", "kind": "pattern_tuple", "original_kind": "t",
        "children": ["pattern1"], "type": UNIT_TYPE, "span": span(1), "provenance": {"origin": "source"}});
    nodes.push(pattern_parts);
    let declaration = nodes
        .iter_mut()
        .find(|node| node["id"] == "statement1")
        .unwrap();
    declaration["children"] = json!(["tuple1", "value1"]);
    function["symbols"].as_array_mut().unwrap().push(
        json!({"id": "s:y", "name": "y", "kind": "local", "scope": "f", "node": "pattern1",
               "mutable": true, "type": INT}),
    );
    assert_eq!(dead_stores(document), [("x".to_owned(), 1)]);
}

#[test]
fn a_frontend_that_emits_no_flow_leaves_a_function_unknown() {
    let mut document = dead_v();
    document["coverage"]["layers"]["cfg"]["status"] = json!("unsupported");
    let Examined::Unknown(reason) = examined(document) else {
        panic!("a function with no flow was read");
    };
    assert!(reason.contains("control flow"), "{reason}");
}

#[test]
fn a_function_whose_aliasing_is_unknown_stays_unknown() {
    let mut document = dead_v();
    document["unit"]["functions"][0]["coverage"] = json!({
        "capabilities": {"aliasing": {"status": "unknown", "description": "outside the typed build"}},
    });
    let Examined::Unknown(reason) = examined(document) else {
        panic!("a function with unknown aliasing was read");
    };
    assert!(reason.contains("alias"), "{reason}");
}

#[test]
fn code_a_compiler_couldnt_parse_leaves_a_function_unknown() {
    let mut document = dead_v();
    document["coverage"]["layers"]["ast"]["status"] = json!("approximated");
    let Examined::Unknown(reason) = examined(document) else {
        panic!("a function with missing syntax was read");
    };
    assert!(reason.contains("syntax"), "{reason}");
}

#[test]
fn a_construct_the_frontend_doesnt_model_leaves_a_function_unknown() {
    let mut document = dead_v();
    let nodes = document["unit"]["functions"][0]["nodes"]
        .as_array_mut()
        .unwrap();
    nodes
        .iter_mut()
        .find(|node| node["id"] == "value2")
        .unwrap()["kind"] = json!("unsupported");
    let Examined::Unknown(reason) = examined(document) else {
        panic!("a function with an unmodeled construct was read");
    };
    assert!(reason.contains("does not model"), "{reason}");
}

#[test]
fn accesses_a_frontend_doesnt_record_leave_a_function_unknown() {
    let mut document = dead_v();
    document["coverage"]["layers"]["symbol"]["status"] = json!("approximated");
    document["unit"]["functions"][0]["accesses"] = json!([]);
    let Examined::Unknown(reason) = examined(document) else {
        panic!("a function with no accesses was read");
    };
    assert!(reason.contains("no accesses"), "{reason}");
}

#[test]
fn a_rule_can_be_named_by_a_suppression() {
    assert!(crate::suppress::RULE_IDS.contains(&dead_store::RULE));
}
