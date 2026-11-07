//! Which `if`s could be guard clauses, in the profile's terms.

use super::run_facts;
use crate::facts::GuardClause;
use serde_json::{Value, json};

const EXAMPLE: &str = include_str!("testdata/example.cpg.json");

/// The statements of a block, as the kinds they hold.
struct If {
    nested: &'static [&'static str],
    otherwise: Option<&'static [&'static str]>,
    origin: &'static str,
}

impl Default for If {
    fn default() -> Self {
        Self {
            nested: &["call", "call"],
            otherwise: Some(&["return"]),
            origin: "source",
        }
    }
}

struct Nodes {
    nodes: Vec<Value>,
    span: Value,
}

impl Nodes {
    fn add(&mut self, id: &str, kind: &str, children: &[String]) -> String {
        self.add_from(id, (kind, "source"), children)
    }

    fn add_from(&mut self, id: &str, (kind, origin): (&str, &str), children: &[String]) -> String {
        self.nodes
            .push(json!({"id": id, "kind": kind, "original_kind": kind,
                               "children": children, "span": self.span,
                               "provenance": {"origin": origin}}));
        id.to_owned()
    }

    /// A block of those statements, each one under a statement of its own.
    fn block(&mut self, id: &str, statements: &[&str]) -> String {
        let children: Vec<String> = statements
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                let inner = self.add(&format!("{id}.{index}"), kind, &[]);
                self.add(&format!("{id}.{index}.s"), "expression_statement", &[inner])
            })
            .collect();
        self.add(id, "block", &children)
    }

    fn if_node(&mut self, id: &str, shape: &If) -> String {
        let condition = self.add(&format!("{id}.c"), "literal", &[]);
        let nested = self.block(&format!("{id}.then"), shape.nested);
        let mut children = vec![condition, nested];
        if let Some(otherwise) = shape.otherwise {
            children.push(self.block(&format!("{id}.else"), otherwise));
        }
        self.add_from(id, ("if", shape.origin), &children)
    }
}

fn with_root(build: impl FnOnce(&mut Nodes) -> String) -> Vec<GuardClause> {
    let mut document: Value = serde_json::from_str(EXAMPLE).unwrap();
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    functions.truncate(1);
    let function = &mut functions[0];
    let mut nodes = Nodes {
        nodes: Vec::new(),
        span: function["span"].clone(),
    };
    let root = build(&mut nodes);
    function["root"] = json!(root);
    function["nodes"] = json!(nodes.nodes);
    for table in ["refs", "places", "accesses", "symbols"] {
        function[table] = json!([]);
    }
    function["signature"]["parameters"] = json!([]);
    function["signature"]
        .as_object_mut()
        .unwrap()
        .remove("receiver");
    run_facts(vec![document]).unwrap().remove(0).guard_clauses
}

fn guards_of(shape: &If) -> Vec<GuardClause> {
    with_root(|nodes| nodes.if_node("if", shape))
}

#[test]
fn an_else_that_returns_around_a_main_path_could_return_early() {
    let [guard] = &guards_of(&If::default())[..] else {
        panic!("one guard clause");
    };
    assert_eq!(guard.statements, 2);
    assert!(guard.line > 0);
}

#[test]
fn one_nested_statement_is_no_main_path() {
    let found = guards_of(&If {
        nested: &["call"],
        ..If::default()
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_else_that_carries_on_has_nothing_to_return_early() {
    let found = guards_of(&If {
        otherwise: Some(&["call"]),
        ..If::default()
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_if_with_no_else_is_already_flat() {
    let found = guards_of(&If {
        otherwise: None,
        ..If::default()
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn both_sides_returning_has_no_main_path_to_bring_out() {
    let found = guards_of(&If {
        nested: &["call", "return"],
        ..If::default()
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_if_a_lowering_wrote_has_no_author_to_tell() {
    let found = guards_of(&If {
        origin: "desugaring",
        ..If::default()
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_else_if_can_not_return_early_on_its_own() {
    let found = with_root(|nodes| {
        let inner = nodes.if_node("inner", &If::default());
        let condition = nodes.add("outer.c", "literal", &[]);
        let nested = nodes.block("outer.then", &["call", "call"]);
        nodes.add("outer", "if", &[condition, nested, inner])
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_return_with_no_statement_around_it_still_returns() {
    let found = with_root(|nodes| {
        let condition = nodes.add("c", "literal", &[]);
        let nested = nodes.block("then", &["call", "call"]);
        let value = nodes.add("r", "return", &[]);
        let otherwise = nodes.add("else", "block", &[value]);
        nodes.add("if", "if", &[condition, nested, otherwise])
    });
    assert_eq!(found.len(), 1, "{found:?}");
}
