//! The derivations over flows no frontend's lowering shapes: any graph a document can hold.

use super::flow::{Derived, Flow, Reaching, Standing, derive};
use super::model::{Coverage, Function};
use crate::wording::Wording;
use serde_json::{Value, json};

/// What core derives of `function` in Rust's terms.
fn rust_flow(coverage: &Coverage, function: &Function) -> Derived {
    derive(coverage, function, Wording::of("rust"))
}

const ENTRY: &str = "entry";
const EXIT: &str = "exit";

/// A function with these transfers and accesses; every other vertex is a node of a body.
struct Shape<'a> {
    edges: &'a [(&'a str, &'a str, &'a str)],
    accesses: &'a [(&'a str, &'a str, &'a str)],
    parameters: &'a [&'a str],
    /// Compound assignments, each naming the identifier it updates.
    compound: &'a [(&'a str, &'a str)],
}

impl Shape<'_> {
    fn nodes(&self) -> Vec<Value> {
        let node = |id: &str, kind: &str, children: Vec<&str>| {
            json!({"id": id, "kind": kind, "original_kind": kind, "children": children,
                   "provenance": {"origin": "source"}})
        };
        let mut ids: Vec<&str> = self
            .edges
            .iter()
            .flat_map(|(source, target, _)| [*source, *target])
            .chain(self.compound.iter().map(|(assignment, _)| *assignment))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter(|id| *id != ENTRY && *id != EXIT)
            .map(|id| {
                match self
                    .compound
                    .iter()
                    .find(|(assignment, _)| *assignment == id)
                {
                    Some((_, target)) => node(id, "compound_assignment", vec![target]),
                    None if self.compound.iter().any(|(_, target)| *target == id) => {
                        node(id, "identifier", vec![])
                    }
                    None => node(id, "call", vec![]),
                }
            })
            .collect()
    }

    fn function(&self) -> Function {
        let span = json!({"file": "file", "start": 0, "end": 1});
        let edges: Vec<Value> = self
            .edges
            .iter()
            .enumerate()
            .map(|(index, (source, target, kind))| {
                let (kind, role) = match kind.split_once('/') {
                    Some((kind, role)) => (kind, Some(role)),
                    None => (*kind, None),
                };
                let mut edge = json!({"id": format!("e{index}"), "source": source,
                                      "target": target, "kind": kind});
                if let Some(role) = role {
                    edge["loop_role"] = json!(role);
                }
                edge
            })
            .collect();
        let accesses: Vec<Value> = self
            .accesses
            .iter()
            .map(|(node, symbol, mode)| json!({"node": node, "symbol": symbol, "mode": mode}))
            .collect();
        let mut symbols: Vec<&str> = self.accesses.iter().map(|(_, symbol, _)| *symbol).collect();
        symbols.sort_unstable();
        symbols.dedup();
        let symbols: Vec<Value> = symbols
            .into_iter()
            .map(|symbol| {
                let mut record = json!({"id": symbol, "name": symbol, "kind": "local",
                                        "scope": "f", "mutable": true});
                if !self.parameters.contains(&symbol) {
                    record["node"] = json!("declared");
                }
                record
            })
            .collect();
        serde_json::from_value(json!({
            "id": "fn:f", "definition": "def:f", "name": "f", "span": span,
            "signature": {"span": span, "parameters": [], "result": "type:unit", "generics": [],
                          "lifetimes": [], "outlives": [], "modifiers": []},
            "root": "root", "nodes": self.nodes(),
            "cfg": {"entry": ENTRY, "exits": [{"id": EXIT, "kind": "normal"}], "edges": edges},
            "symbols": symbols, "accesses": accesses, "refs": [], "places": [], "values": [],
        }))
        .unwrap()
    }
}

fn bare(edges: &[(&'static str, &'static str, &'static str)]) -> Function {
    Shape {
        edges,
        accesses: &[],
        parameters: &[],
        compound: &[],
    }
    .function()
}

fn parent(flow: &Flow<'_>, tree: &super::flow::Tree, id: &str) -> Option<String> {
    tree.parent(flow.vertex(id).unwrap())
        .map(|parent| flow.id(parent).to_owned())
}

fn control(flow: &Flow<'_>) -> Vec<(String, String, String)> {
    flow.control_dependence(&flow.post_dominators())
        .into_iter()
        .map(|(source, target, kind)| {
            (
                flow.id(source).to_owned(),
                flow.id(target).to_owned(),
                kind.to_owned(),
            )
        })
        .collect()
}

fn triple(source: &str, target: &str, kind: &str) -> (String, String, String) {
    (source.to_owned(), target.to_owned(), kind.to_owned())
}

const DIAMOND: &[(&str, &str, &str)] = &[
    (ENTRY, "branch", "normal"),
    ("branch", "then", "true"),
    ("branch", "other", "false"),
    ("then", "after", "normal"),
    ("other", "after", "normal"),
    ("after", EXIT, "normal"),
];

#[test]
fn a_diamond_joins_at_what_both_of_its_sides_reach() {
    let function = bare(DIAMOND);
    let flow = Flow::of(&function).unwrap();
    let dominators = flow.dominators();
    let post = flow.post_dominators();

    assert_eq!(parent(&flow, &dominators, "after").unwrap(), "branch");
    assert_eq!(parent(&flow, &dominators, "then").unwrap(), "branch");
    assert_eq!(parent(&flow, &post, "branch").unwrap(), "after");
    assert_eq!(parent(&flow, &post, "then").unwrap(), "after");
    assert!(dominators.contains(flow.vertex("branch").unwrap(), flow.vertex(EXIT).unwrap()));
    assert!(post.contains(flow.vertex("after").unwrap(), flow.vertex("then").unwrap()));
    let edges = control(&flow);
    assert_eq!(edges.len(), 2);
    assert!(edges.contains(&triple("branch", "then", "true")));
    assert!(edges.contains(&triple("branch", "other", "false")));
}

#[test]
fn what_follows_an_early_return_depends_on_the_branch_that_did_not_take_it() {
    let function = bare(&[
        (ENTRY, "branch", "normal"),
        ("branch", "leave", "true"),
        ("leave", EXIT, "return"),
        ("branch", "stay", "false"),
        ("stay", EXIT, "normal"),
    ]);
    let flow = Flow::of(&function).unwrap();
    let edges = control(&flow);

    assert!(edges.contains(&triple("branch", "stay", "false")));
    assert!(edges.contains(&triple("branch", "leave", "true")));
}

#[test]
fn a_loop_body_depends_on_its_head_and_the_head_on_itself() {
    let function = bare(&[
        (ENTRY, "head", "normal"),
        ("head", "body", "true"),
        ("body", "head", "normal/back_edge"),
        ("head", "after", "false/loop_exit"),
        ("after", EXIT, "normal"),
    ]);
    let flow = Flow::of(&function).unwrap();
    let edges = control(&flow);

    assert!(edges.contains(&triple("head", "body", "true")));
    assert!(edges.contains(&triple("head", "head", "true")));
    assert!(!edges.iter().any(|(_, target, _)| target == "after"));
    assert_eq!(
        flow.back_edges(),
        vec![(flow.vertex("body").unwrap(), flow.vertex("head").unwrap())]
    );
}

#[test]
fn a_loop_nothing_leaves_and_a_call_that_never_returns_still_have_post_dominators() {
    let function = bare(&[
        (ENTRY, "branch", "normal"),
        ("branch", "spin", "true"),
        ("spin", "more", "normal"),
        ("more", "spin", "normal/back_edge"),
        ("branch", "panic", "false"),
    ]);
    let flow = Flow::of(&function).unwrap();
    let post = flow.post_dominators();

    assert_eq!(parent(&flow, &post, "branch"), None);
    assert_eq!(parent(&flow, &post, "spin").unwrap(), "more");
    assert_eq!(parent(&flow, &post, "panic"), None);
    let edges = control(&flow);
    assert!(edges.contains(&triple("branch", "panic", "false")));
    assert!(edges.contains(&triple("branch", "spin", "true")));
}

#[test]
fn what_follows_a_loop_depends_on_the_loop_finishing_and_is_marked_so() {
    let function = bare(&[
        (ENTRY, "head", "normal"),
        ("head", "body", "true"),
        ("body", "head", "normal/back_edge"),
        ("head", "after", "false/loop_exit"),
        ("after", "last", "normal"),
        ("last", EXIT, "normal"),
    ]);
    let derived = rust_flow(&coverage("supported", "supported"), &function);
    let termination: Vec<(&str, &str, &str)> = derived
        .control
        .iter()
        .filter(|edge| edge.termination)
        .map(|edge| {
            (
                edge.source.as_str(),
                edge.kind.as_str(),
                edge.target.as_str(),
            )
        })
        .collect();

    assert_eq!(
        termination,
        vec![("head", "false", "after"), ("head", "false", "last")]
    );
    assert!(
        derived
            .control
            .iter()
            .any(|edge| !edge.termination && edge.target == "body")
    );
}

#[test]
fn without_a_loop_nothing_depends_on_one_finishing() {
    let function = bare(DIAMOND);
    let derived = rust_flow(&coverage("supported", "supported"), &function);

    assert!(derived.control.iter().all(|edge| !edge.termination));
}

#[test]
fn a_flow_with_two_ways_into_a_loop_keeps_exact_dominators() {
    // A frontend for a language with `goto` can emit this; a structured lowering cannot.
    let function = bare(&[
        (ENTRY, "branch", "normal"),
        ("branch", "a", "true"),
        ("branch", "b", "false"),
        ("a", "b", "normal"),
        ("b", "a", "normal"),
        ("a", EXIT, "normal"),
    ]);
    let flow = Flow::of(&function).unwrap();
    let dominators = flow.dominators();

    assert_eq!(parent(&flow, &dominators, "a").unwrap(), "branch");
    assert_eq!(parent(&flow, &dominators, "b").unwrap(), "branch");
}

#[test]
fn a_vertex_the_entry_never_reaches_has_no_facts() {
    let function = bare(&[
        (ENTRY, "live", "normal"),
        ("live", EXIT, "normal"),
        ("dead", "live", "normal"),
    ]);
    let flow = Flow::of(&function).unwrap();

    assert_eq!(parent(&flow, &flow.dominators(), "live"), None);
    assert!(!flow.order().contains(&flow.vertex("dead").unwrap()));
}

fn data(shape: &Shape<'_>) -> Vec<(String, String, String)> {
    let function = shape.function();
    let flow = Flow::of(&function).unwrap();
    Reaching::of(&function, &flow)
        .data_dependence()
        .into_iter()
        .map(|(source, target, symbol)| {
            (
                flow.id(source).to_owned(),
                flow.id(target).to_owned(),
                symbol.to_owned(),
            )
        })
        .collect()
}

const LINE: &[(&str, &str, &str)] = &[
    (ENTRY, "first", "normal"),
    ("first", "second", "normal"),
    ("second", "use", "normal"),
    ("use", EXIT, "normal"),
];

#[test]
fn an_overwrite_hides_what_came_before_it() {
    let edges = data(&Shape {
        edges: LINE,
        accesses: &[
            ("first", "x", "write"),
            ("second", "x", "write"),
            ("use", "x", "read"),
        ],
        parameters: &[],
        compound: &[],
    });

    assert_eq!(edges, vec![triple("second", "use", "x")]);
}

#[test]
fn a_write_through_a_borrow_keeps_what_came_before_it() {
    let edges = data(&Shape {
        edges: LINE,
        accesses: &[
            ("first", "x", "write"),
            ("second", "x", "read_write"),
            ("use", "x", "read"),
        ],
        parameters: &[],
        compound: &[],
    });

    assert!(edges.contains(&triple("first", "use", "x")));
    assert!(edges.contains(&triple("second", "use", "x")));
    assert!(edges.contains(&triple("first", "second", "x")));
}

#[test]
fn a_compound_assignment_replaces_what_it_reads() {
    let edges = data(&Shape {
        edges: LINE,
        accesses: &[
            ("first", "x", "write"),
            ("second", "x", "read_write"),
            ("use", "x", "read"),
        ],
        parameters: &[],
        compound: &[("assign", "second")],
    });

    assert!(edges.contains(&triple("first", "second", "x")));
    assert!(edges.contains(&triple("second", "use", "x")));
    assert!(!edges.contains(&triple("first", "use", "x")));
}

#[test]
fn both_sides_of_a_branch_reach_the_join_and_a_parameter_reaches_from_entry() {
    let edges = data(&Shape {
        edges: DIAMOND,
        accesses: &[
            ("then", "x", "write"),
            ("after", "x", "read"),
            ("after", "p", "read"),
        ],
        parameters: &["p"],
        compound: &[],
    });

    assert!(edges.contains(&triple("then", "after", "x")));
    assert!(edges.contains(&triple(ENTRY, "after", "p")));
    assert_eq!(edges.len(), 2);
}

#[test]
fn a_write_in_a_loop_reaches_its_own_next_iteration() {
    let edges = data(&Shape {
        edges: &[
            (ENTRY, "init", "normal"),
            ("init", "head", "normal"),
            ("head", "body", "true"),
            ("body", "head", "normal/back_edge"),
            ("head", EXIT, "false/loop_exit"),
        ],
        accesses: &[
            ("init", "x", "write"),
            ("body", "x", "read_write"),
            ("head", "x", "read"),
        ],
        parameters: &[],
        compound: &[],
    });

    assert!(edges.contains(&triple("init", "head", "x")));
    assert!(edges.contains(&triple("body", "head", "x")));
    assert!(edges.contains(&triple("body", "body", "x")));
    assert!(edges.contains(&triple("init", "body", "x")));
}

fn coverage(cfg: &str, aliasing: &str) -> Coverage {
    let declare = |status: &str| json!({"status": status, "description": "stated"});
    serde_json::from_value(json!({
        "capabilities": {"aliasing": declare(aliasing), "exceptional-flow": declare("unsupported")},
        "layers": {"cfg": declare(cfg)},
    }))
    .unwrap()
}

#[test]
fn a_frontend_that_emits_no_flow_leaves_every_function_unknown_not_empty() {
    let function = bare(&[]);
    let derived = rust_flow(&coverage("supported", "supported"), &function);
    assert_eq!(derived.standing, Standing::Unknown);
    assert!(derived.data.is_none());

    let function = bare(DIAMOND);
    let derived = rust_flow(&coverage("unsupported", "supported"), &function);
    assert_eq!(derived.standing, Standing::Unknown);
    assert!(derived.control.is_empty());
}

#[test]
fn flow_without_accesses_keeps_control_and_says_data_is_unknown() {
    let function = Shape {
        edges: DIAMOND,
        accesses: &[("after", "x", "read")],
        parameters: &[],
        compound: &[],
    }
    .function();
    let mut without = function.clone();
    without.accesses.clear();

    let derived = rust_flow(&coverage("supported", "supported"), &without);
    assert_eq!(derived.standing, Standing::Approximated);
    assert!(derived.data.is_none());
    assert_eq!(derived.control.len(), 2);
    assert_eq!(derived.execution.len(), 3);
}

#[test]
fn a_binding_a_raw_pointer_reaches_can_change_through_it() {
    let shape = Shape {
        edges: LINE,
        accesses: &[
            ("first", "x", "write"),
            // Taking an address reads the binding; a read and write would say more than that.
            ("second", "x", "read"),
            ("use", "x", "read"),
        ],
        parameters: &[],
        compound: &[],
    };
    let with_place = |mode: &str, wording| {
        let mut function = shape.function();
        function.places = vec![
            serde_json::from_value(json!({
                "id": "p", "node": "second", "root": {"symbol": "x"}, "path": [],
                "access": "read", "mode": mode,
            }))
            .unwrap(),
        ];
        derive(&coverage("supported", "supported"), &function, wording)
    };

    let shared = with_place("shared", Wording::of("rust"));
    assert_eq!(shared.standing, Standing::Known, "{:?}", shared.reasons);

    let raw = with_place("raw", Wording::of("rust"));
    assert_eq!(raw.standing, Standing::Approximated);
    assert!(
        raw.reasons
            .iter()
            .any(|reason| reason.contains("a borrow, a capture"))
    );
    assert!(raw.data.unwrap().iter().all(|edge| edge.approximate));

    let in_cxx = with_place("raw", Wording::of("cxx")).reasons;
    let expected = "x can also change through a reference, a pointer, a lambda capture or a write to part of it";
    assert!(in_cxx.iter().any(|reason| reason == expected), "{in_cxx:?}");
}

#[test]
fn what_a_frontend_cannot_say_about_aliasing_is_recorded_and_unwinding_assumed_away() {
    let function = bare(DIAMOND);
    let known = rust_flow(&coverage("supported", "supported"), &function);
    assert_eq!(known.standing, Standing::Known);
    assert_eq!(known.assumptions.len(), 1);

    let unstated = rust_flow(&coverage("supported", "unknown"), &function);
    assert_eq!(unstated.standing, Standing::Approximated);
    assert_eq!(unstated.reasons.len(), 1);
}
