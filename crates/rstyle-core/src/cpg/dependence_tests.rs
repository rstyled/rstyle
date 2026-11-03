//! Dependence graphs derived from the example document. The fixture crate's
//! round trip against the driver (`cpg compare`) covers every other construct.

use super::dependence::Roles;
use super::{all_facts, layout};
use crate::facts::{EdgeKind, FnFacts, NodeKind, Pdg, PdgEdge};
use serde_json::{Value, json};

const EXAMPLE: &str = include_str!("testdata/example.cpg.json");

fn example() -> Value {
    serde_json::from_str(EXAMPLE).unwrap()
}

fn derived(document: Value) -> Result<Vec<FnFacts>, String> {
    all_facts(&layout::bundle(vec![document])?)
}

fn graphs(document: Value) -> Vec<FnFacts> {
    derived(document).unwrap()
}

fn graph(facts: &FnFacts) -> &Pdg {
    facts.pdg.as_ref().unwrap()
}

fn node<'d>(document: &'d mut Value, id: &str) -> &'d mut Value {
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    functions
        .iter_mut()
        .flat_map(|function| function["nodes"].as_array_mut().unwrap())
        .find(|node| node["id"] == id)
        .unwrap()
}

fn edge(from: usize, to: usize, kind: EdgeKind, arg_pos: usize) -> PdgEdge {
    PdgEdge {
        from,
        to,
        kind,
        arg_pos,
    }
}

#[test]
fn rule_facts_leave_graphs_out() {
    let facts = super::run_facts(vec![example()]).unwrap();
    assert!(facts.iter().all(|facts| facts.pdg.is_none()));
}

#[test]
fn calls_take_their_arguments_and_the_body_returns_its_value() {
    let facts = graphs(example());
    let top = graph(&facts[1]);
    let kinds: Vec<_> = top.nodes.iter().map(|node| node.kind).collect();
    use NodeKind::*;
    assert_eq!(kinds, [Param, Param, Call, Call, Lit, Return]);
    assert_eq!(
        top.nodes[2].callee_id.as_deref(),
        Some("core::cmp::Ord::min")
    );
    assert_eq!(top.nodes[3].callee_id.as_deref(), Some("example::larger"));
    let sig = top.nodes[3].callee_sig.as_ref().unwrap();
    assert_eq!((sig.params.len(), sig.generics), (2, 1));
    assert_eq!(
        (
            top.nodes[4].lit_kind.as_deref(),
            top.nodes[4].detail.as_str()
        ),
        (Some("path"), "LIMIT")
    );
    use EdgeKind::Data;
    assert_eq!(
        top.edges,
        [
            edge(0, 3, Data, 0),
            edge(1, 3, Data, 1),
            edge(2, 5, Data, 0),
            edge(3, 2, Data, 0),
            edge(4, 2, Data, 1),
        ]
    );
}

#[test]
fn iterations_bind_their_items_and_control_their_bodies() {
    let facts = graphs(example());
    let count = graph(&facts[2]);
    let kinds: Vec<_> = count.nodes.iter().map(|node| node.kind).collect();
    use NodeKind::*;
    assert_eq!(
        kinds,
        [
            Param, Lit, Iterate, Branch, Op, Op, Lit, Op, Lit, Op, Lit, Return
        ]
    );
    let details: Vec<_> = count
        .nodes
        .iter()
        .map(|node| node.detail.as_str())
        .collect();
    assert_eq!(
        details,
        [
            "", "0", "", "", "&&", ">", "LIMIT", "<", "10", "+=", "1", ""
        ]
    );
    use EdgeKind::{Ctrl, Data};
    for expected in [
        // The items come from the parameter, and `*item` reads the iteration.
        edge(0, 2, Data, 0),
        edge(2, 5, Data, 0),
        // The branch is inside the loop body; the increment inside the branch.
        edge(2, 3, Ctrl, 0),
        edge(3, 9, Ctrl, 0),
        // `total += 1`: the value before the place it writes.
        edge(10, 9, Data, 0),
        edge(1, 9, Data, 1),
        // The body returns `total`, bound to its initializer.
        edge(1, 11, Data, 0),
    ] {
        assert!(
            count.edges.contains(&expected),
            "{expected:?} in {:?}",
            count.edges
        );
    }
}

#[test]
fn literals_show_their_values() {
    let detail = |value: Value, origin: &str| {
        let mut document = example();
        let literal = node(&mut document, "count.one");
        literal["value"] = value;
        if origin != "source" {
            literal["provenance"] = json!({"origin": origin, "construct": "for"});
        }
        let facts = graphs(document);
        let node = &graph(&facts[2]).nodes[10];
        (node.lit_kind.clone().unwrap(), node.detail.clone())
    };
    let pair = |kind: &str, detail: &str| (kind.to_string(), detail.to_string());
    let long = "x".repeat(40);
    let string = json!({"kind": "string", "value": long});
    assert_eq!(detail(string, "source").0, "str");
    let short = json!({"kind": "string", "value": "a\n"});
    assert_eq!(detail(short, "source"), pair("str", "\"a\\n\""));
    assert_eq!(detail(json!({"kind": "string"}), "source"), pair("str", ""));
    assert_eq!(
        detail(json!({"kind": "char", "value": "z"}), "source"),
        pair("char", "'z'")
    );
    assert_eq!(
        detail(json!({"kind": "bool", "value": "true"}), "source"),
        pair("bool", "true")
    );
    assert_eq!(
        detail(json!({"kind": "float", "value": "1.5"}), "source"),
        pair("float", "")
    );
    assert_eq!(
        detail(json!({"kind": "integer", "value": "7"}), "desugaring"),
        pair("int", "")
    );
}

#[test]
fn roles_come_from_the_producer_namespace() {
    assert!(Roles::of("rust").views().contains(&"iter".to_string()));
    assert!(Roles::of("go").views().is_empty());
    assert_eq!(
        Roles::parse("schema = \"other/1\"\nnamespace = \"x\"").unwrap_err(),
        "Unsupported dependence roles schema other/1"
    );
    assert!(
        Roles::parse("schema = \"rstyle.dependence-roles/1\"\nnamespace = \"x\"\nextra = 1")
            .is_err()
    );
}

#[test]
fn graph_inputs_must_be_present() {
    let error = |change: fn(&mut Value)| {
        let mut document = example();
        change(&mut document);
        derived(document).unwrap_err()
    };
    assert_eq!(
        error(|d| {
            node(d, "top.min").as_object_mut().unwrap().remove("type");
        }),
        "Node top.min has no type"
    );
    assert_eq!(
        error(|d| {
            let parameter = &mut d["unit"]["functions"][1]["signature"]["parameters"][0];
            parameter.as_object_mut().unwrap().remove("span");
        }),
        "Parameter 0 has no span"
    );
    assert_eq!(
        error(|d| {
            let symbols = d["unit"]["functions"][1]["symbols"].as_array_mut().unwrap();
            symbols[0].as_object_mut().unwrap().remove("type");
        }),
        "Symbol top.a has no type"
    );
}
