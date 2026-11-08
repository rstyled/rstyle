//! What a call's place and type say of its result, in the profile's terms.

use super::super::super::layout::bundle;
use super::super::super::source::Sources;
use super::super::super::tree::Tables;
use super::super::{Examined, examine};
use super::*;
use crate::config::Percent;
use serde_json::{Value, json};

const INT: &str = "type:int";
const UNIT: &str = "type:unit";
const FILE: &str = "file:a";

fn node(id: &str, (kind, line): (&str, usize), children: &[&str], origin: &str) -> Value {
    let start = line * 10;
    json!({"id": id, "kind": kind, "original_kind": kind, "children": children, "type": UNIT,
           "span": {"file": FILE, "start": start, "end": start + 5},
           "provenance": {"origin": origin}})
}

fn document_of(function: Value) -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../../testdata/example.cpg.json")).unwrap();
    let unit = &mut document["unit"];
    unit["functions"] = json!([function]);
    unit["definitions"] = json!([
        {"id": "def:f", "kind": "function", "name": "f", "stable_identity": "f", "scope": "unit"},
        {"id": "def:g", "kind": "function", "name": "g", "stable_identity": "id:g", "scope": "unit"},
        {"id": "def:t", "kind": "type", "name": "T", "stable_identity": "T", "scope": "unit"},
    ]);
    unit["types"] = json!([
        {"id": UNIT, "kind": "unit", "spelling": "()"},
        {"id": INT, "kind": "integer", "spelling": "int", "bits": 64, "signed": true},
        {"id": "type:never", "kind": "bottom", "spelling": "!"},
        {"id": "type:opaque", "kind": "opaque", "spelling": "unknown"},
        {"id": "type:parameter", "kind": "parameter", "spelling": "P", "index": 0},
        {"id": "type:t", "kind": "record", "spelling": "T", "definition": "def:t"},
        {"id": "type:ref", "kind": "pointer", "spelling": "*T", "target": "type:t", "mode": "raw"},
        {"id": "type:loop", "kind": "pointer", "spelling": "*L", "target": "type:pool", "mode": "raw"},
        {"id": "type:pool", "kind": "pointer", "spelling": "**L", "target": "type:loop", "mode": "raw"},
    ]);
    unit["files"] = json!([{"id": FILE, "path": "a.go", "digest": "", "origin": "source",
                            "content": "ok .....\n ".repeat(20)}]);
    document
}

/// A function of one call to `g`, of type `ty`, under `parent`: a node of that kind and origin, on
/// a block's first line.
struct Fixture {
    parent: (&'static str, &'static str),
    ty: &'static str,
    reference: Value,
    receiver: Option<&'static str>,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            parent: ("expression_statement", "source"),
            ty: INT,
            reference: json!({"node": "call", "status": "resolved", "dispatch": "static",
                              "target": {"kind": "definition", "id": "def:g"}}),
            receiver: None,
        }
    }
}

impl Fixture {
    /// The body's nodes and the order they run in.
    fn body(&self) -> (Vec<Value>, Vec<Value>) {
        let (parent, origin) = self.parent;
        let children: &[&str] = match self.receiver {
            Some(_) => &["receiver"],
            None => &[],
        };
        let mut call = node("call", ("call", 1), children, "source");
        call["type"] = json!(self.ty);
        let mut nodes = vec![
            node("root", ("block", 0), &["statement"], "source"),
            node("statement", (parent, 1), &["call"], origin),
            call,
        ];
        let mut edges = vec![
            json!({"id": "e0", "source": "entry", "target": "call", "kind": "normal"}),
            json!({"id": "e1", "source": "call", "target": "statement", "kind": "normal"}),
            json!({"id": "e2", "source": "statement", "target": "exit", "kind": "normal"}),
        ];
        if let Some(receiver) = self.receiver {
            let mut value = node("receiver", ("identifier", 1), &[], "source");
            value["type"] = json!(receiver);
            nodes.push(value);
            edges[0] =
                json!({"id": "e0", "source": "entry", "target": "receiver", "kind": "normal"});
            edges.push(
                json!({"id": "e3", "source": "receiver", "target": "call", "kind": "normal"}),
            );
        }
        (nodes, edges)
    }

    fn document(self) -> Value {
        let (nodes, edges) = self.body();
        let function = json!({
            "id": "fn:f", "definition": "def:f", "name": "f",
            "span": {"file": FILE, "start": 0, "end": 5},
            "signature": {"span": {"file": FILE, "start": 0, "end": 5}, "parameters": [],
                          "result": UNIT, "generics": [], "lifetimes": [], "outlives": [],
                          "modifiers": []},
            "root": "root", "nodes": nodes,
            "cfg": {"entry": "entry", "exits": [{"id": "exit", "kind": "normal"}], "edges": edges},
            "symbols": [], "accesses": [], "refs": [self.reference], "places": [], "values": [],
        });
        document_of(function)
    }

    fn results(self) -> Vec<ResultUse> {
        let document = bundle(vec![self.document()]).unwrap();
        let unit = document.unit.as_ref().unwrap();
        let tables = Tables::new(unit).unwrap();
        let sources = Sources::new(unit).unwrap();
        let function = &unit.functions.as_ref().unwrap()[0];
        match examine((&document.coverage, "test"), function, &tables, &sources) {
            Examined::Read { results, .. } => results,
            Examined::Unknown(reason) => panic!("the function was not read: {reason}"),
        }
    }
}

fn method_on(receiver: &'static str, ty: &'static str) -> Fixture {
    Fixture {
        ty,
        receiver: Some(receiver),
        reference: json!({"node": "call", "status": "resolved", "dispatch": "method",
                          "target": {"kind": "definition", "id": "def:g"}}),
        ..Fixture::default()
    }
}

#[test]
fn a_call_that_is_all_of_a_statement_drops_its_result() {
    let [call] = &Fixture::default().results()[..] else {
        panic!("one call counts");
    };
    assert!(call.dropped);
    assert_eq!(call.callee, "id:g");
    assert_eq!(call.name, "g");
    assert_eq!(call.line, 2);
}

#[test]
fn a_call_anything_else_holds_uses_its_result() {
    let results = Fixture {
        parent: ("return", "source"),
        ..Fixture::default()
    }
    .results();
    let [call] = &results[..] else {
        panic!("one call counts");
    };
    assert!(!call.dropped);
}

#[test]
fn a_statement_a_lowering_made_up_says_nothing() {
    let results = Fixture {
        parent: ("expression_statement", "desugaring"),
        ..Fixture::default()
    }
    .results();
    assert!(results.is_empty(), "{results:?}");
}

#[test]
fn an_extension_says_how_a_call_runs_and_nothing_of_its_result() {
    let results = Fixture {
        parent: ("extension", "source"),
        ..Fixture::default()
    }
    .results();
    assert!(results.is_empty(), "{results:?}");
}

#[test]
fn a_call_with_nothing_of_its_own_to_give_does_not_count() {
    for ty in [UNIT, "type:never", "type:opaque", "type:parameter"] {
        let results = Fixture {
            ty,
            ..Fixture::default()
        }
        .results();
        assert!(results.is_empty(), "{ty}: {results:?}");
    }
}

#[test]
fn a_method_returning_its_receiver_is_there_to_chain() {
    let on_a_value = method_on("type:t", "type:ref").results();
    assert!(on_a_value.is_empty(), "{on_a_value:?}");
    let on_a_pointer = method_on("type:ref", "type:ref").results();
    assert!(on_a_pointer.is_empty(), "{on_a_pointer:?}");
    assert_eq!(method_on("type:t", INT).results().len(), 1);
}

#[test]
fn a_call_that_names_nothing_does_not_count() {
    for reference in [
        json!({"node": "call", "status": "unresolved", "reason": "missing"}),
        json!({"node": "call", "status": "resolved", "dispatch": "function_value",
               "target": {"kind": "symbol", "id": "s:f"}}),
        json!({"node": "call", "status": "resolved", "target": {"kind": "definition", "id": "def:t"}}),
    ] {
        let results = Fixture {
            reference,
            ..Fixture::default()
        }
        .results();
        assert!(results.is_empty(), "{results:?}");
    }
}

#[test]
fn a_dependent_call_is_a_call_of_what_it_goes_through() {
    let results = Fixture {
        reference: json!({"node": "call", "status": "dependent", "via": "def:g"}),
        ..Fixture::default()
    }
    .results();
    assert_eq!(results[0].callee, "id:g");
}

#[test]
fn a_callee_needs_enough_calls_nearly_all_of_which_use_it() {
    let default = IgnoredResultConfig::default();
    assert!(stands_out(9, 10, &default));
    assert!(!stands_out(3, 4, &default), "too few calls");
    assert!(!stands_out(8, 10, &default), "too many drop it");
}

#[test]
fn a_project_says_how_many_calls_and_how_much_of_them_it_takes() {
    let limits = IgnoredResultConfig {
        min_calls: 4,
        min_used_percent: Percent(75),
    };
    assert!(stands_out(3, 4, &limits));
    assert!(!stands_out(2, 3, &limits), "too few calls");
    assert!(!stands_out(5, 7, &limits), "too many drop it");
}

#[test]
fn a_chain_of_pointers_that_repeats_ends_where_it_repeats() {
    let results = method_on("type:loop", INT).results();
    assert_eq!(results.len(), 1, "{results:?}");
}

#[test]
fn a_function_whose_calls_resolve_to_nothing_known_is_unknown() {
    let mut document = Fixture::default().document();
    document["coverage"]["capabilities"]["resolution"]["status"] = json!("unknown");
    let document = bundle(vec![document]).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let (tables, sources) = (Tables::new(unit).unwrap(), Sources::new(unit).unwrap());
    let function = &unit.functions.as_ref().unwrap()[0];
    let Examined::Unknown(reason) =
        examine((&document.coverage, "test"), function, &tables, &sources)
    else {
        panic!("a function whose calls resolve to nothing known was read");
    };
    assert!(reason.contains("resolve"), "{reason}");
}

#[test]
fn a_rule_can_be_named_by_a_suppression() {
    assert!(crate::suppress::RULE_IDS.contains(&RULE));
}
