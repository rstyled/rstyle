//! Boundary cases in evidence supplied by a frontend.

use super::*;
use crate::cpg::defects::{Reading, parents};
use crate::cpg::flow::{self, Reaching, Vertex};
use crate::cpg::layout::bundle;
use crate::cpg::source::Sources;
use crate::cpg::tree::{Body, Tables};
use serde_json::{Value, json};
use std::rc::Rc;

fn document() -> Value {
    serde_json::from_str(include_str!("../../testdata/deviant-argument-go.cpg.json")).unwrap()
}

fn with_reading(document: Value, index: usize, check: impl FnOnce(&Reading<'_>)) {
    let document = bundle(vec![document]).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let function = &unit.functions.as_ref().unwrap()[index];
    let tables = Tables::new(unit).unwrap();
    let sources = Sources::new(unit).unwrap();
    let flow = flow::with_accesses(&document.coverage, function).unwrap();
    let reaching = Reaching::of(function, &flow);
    let reading = Reading {
        function,
        dependence: reaching.data_dependence(),
        flow,
        reaching,
        body: Body::new(function).unwrap(),
        parents: parents(function),
        tables: &tables,
        sources: &sources,
    };
    check(&reading);
}

#[test]
fn missing_type_metadata_supplies_no_field_evidence() {
    let mut document = document();
    let function = &mut document["unit"]["functions"][19];
    for node in function["nodes"].as_array_mut().unwrap() {
        node.as_object_mut().unwrap().remove("type");
    }
    for symbol in function["symbols"].as_array_mut().unwrap() {
        symbol.as_object_mut().unwrap().remove("type");
    }
    with_reading(document, 19, |reading| {
        let arguments = of(reading).unwrap();
        assert!(arguments.fields.is_empty());
        assert!(arguments.declared.is_empty());
        assert!(!arguments.arguments.is_empty());
    });
}

#[test]
fn recursive_embedded_fields_terminate_and_keep_selectable_names() {
    let mut document = document();
    let types = document["unit"]["types"].as_array_mut().unwrap();
    let writer = types.iter_mut().find(|ty| ty["id"] == "t36").unwrap();
    writer["fields"].as_array_mut().unwrap().push(json!({
        "name": "Writer", "type": "t36", "embedded": true
    }));
    with_reading(document, 19, |reading| {
        let writer = reading.tables.ty("t36").unwrap();
        assert!(!test_suite(reading, writer).unwrap());
        let fields = selectable(reading, writer).unwrap();
        assert!(fields.contains("Writer") && fields.contains("Log"));
    });
}

#[test]
fn a_call_without_a_span_supplies_no_arguments() {
    with_reading(document(), 19, |reading| {
        let mut call = reading.body.node("f19.n30").unwrap().clone();
        call.span = None;
        assert!(
            arguments_of(&mut Lineage::new(reading), &call, ("record", "record"))
                .unwrap()
                .is_empty()
        );
    });
}

#[test]
fn a_call_outside_the_flow_and_any_closure_supplies_no_arguments() {
    with_reading(document(), 19, |reading| {
        let mut call = reading.body.node("f19.n30").unwrap().clone();
        call.id = "detached".into();
        assert!(
            arguments_of(&mut Lineage::new(reading), &call, ("record", "record"))
                .unwrap()
                .is_empty()
        );
    });
}

#[test]
fn an_argument_with_an_invalid_source_span_returns_an_error() {
    let mut document = document();
    document["unit"]["functions"][19]["nodes"][24]["kind"] = json!("literal");
    document["unit"]["functions"][19]["nodes"][24]["span"]["file"] = json!("missing");
    with_reading(document, 19, |reading| {
        let call = reading.body.node("f19.n30").unwrap();
        let error =
            arguments_of(&mut Lineage::new(reading), call, ("record", "record")).unwrap_err();
        assert!(error.contains("missing"), "{error}");
    });
}

#[test]
fn an_argument_with_an_unknown_type_returns_an_error() {
    let mut document = document();
    document["unit"]["functions"][19]["nodes"][28]["type"] = json!("missing");
    with_reading(document, 19, |reading| {
        let call = reading.body.node("f19.n30").unwrap();
        assert_eq!(
            arguments_of(&mut Lineage::new(reading), call, ("record", "record")).unwrap_err(),
            "Unknown type missing"
        );
    });
}

#[test]
fn source_text_without_a_span_is_empty() {
    with_reading(document(), 19, |reading| {
        let mut node = reading.body.node("f19.n0").unwrap().clone();
        node.span = None;
        assert_eq!(source_text(reading, &node).unwrap(), "");
    });
}

#[test]
fn indexed_and_referenced_values_keep_their_base_variable() {
    with_reading(document(), 19, |reading| {
        for kind in [NodeKind::IndexAccess, NodeKind::Reference] {
            let mut node = reading.body.node("f19.n4").unwrap().clone();
            node.kind = kind;
            assert_eq!(
                variable_of(reading, &node).unwrap(),
                (Some(("f19.s2".into(), "self".into())), None)
            );
        }
    });
}

#[test]
fn a_selector_without_a_base_reads_no_variable() {
    with_reading(document(), 19, |reading| {
        let mut node = reading.body.node("f19.n4").unwrap().clone();
        node.children.clear();
        assert_eq!(variable_of(reading, &node).unwrap(), (None, None));
    });
}

#[test]
fn closure_local_sources_include_reads_in_their_declaration() {
    let mut document = document();
    let function = &mut document["unit"]["functions"][24];
    // Model startLegacy := startStorage inside the closure.
    function["nodes"][33]["kind"] = json!("identifier");
    function["nodes"][33]["children"] = json!([]);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "f24.n33");
    function["refs"].as_array_mut().unwrap().push(json!({
        "node": "f24.n33", "status": "resolved", "target": {"kind": "symbol", "id": "f24.s5"}
    }));
    function["accesses"].as_array_mut().unwrap().push(json!({
        "node": "f24.n33", "symbol": "f24.s5", "mode": "read"
    }));
    with_reading(document, 24, |reading| {
        let call = reading.body.node("f24.n43").unwrap();
        let arguments =
            arguments_of(&mut Lineage::new(reading), call, ("record", "record")).unwrap();
        assert!(
            arguments[3].sources.contains("startStorage"),
            "{arguments:?}"
        );
    });
}

#[test]
fn repeated_dependency_paths_are_visited_once() {
    with_reading(document(), 19, |reading| {
        let vertex = reading.flow.vertex("f19.n30").unwrap();
        let once = derived(&mut Lineage::new(reading), &[("f19.s4", vertex)]).unwrap();
        assert!(!once.is_empty());
        let twice = derived(&mut Lineage::new(reading), &[("f19.s4", vertex); 2]).unwrap();
        assert_eq!(twice, once);
    });
}

#[test]
fn sources_worked_out_after_others_match_those_worked_out_alone() {
    with_reading(document(), 19, |reading| {
        let seeds: Vec<(&str, Vertex)> = reading
            .reaching
            .definitions
            .iter()
            .map(|definition| (definition.symbol, definition.vertex))
            .collect();
        let mut shared = Lineage::new(reading);
        for seed in &seeds {
            let alone = derived(&mut Lineage::new(reading), &[*seed]).unwrap();
            assert_eq!(derived(&mut shared, &[*seed]).unwrap(), alone, "{seed:?}");
        }
    });
}

#[test]
fn a_value_a_loop_rewrites_is_made_of_itself() {
    let document = serde_json::from_str(include_str!("../../testdata/example.cpg.json")).unwrap();
    // `total += 1` in a loop: each write of `total` reads the one before, through the back edge.
    with_reading(document, 2, |reading| {
        let tail = reading.flow.vertex("count.tail").unwrap();
        let made = derived(&mut Lineage::new(reading), &[("count.total", tail)]).unwrap();
        assert_eq!(made, BTreeSet::from(["total".to_owned()]));
    });
}

#[test]
fn a_loop_of_values_closes_as_one_component_after_what_it_reaches() {
    // 0 is made of 1, 1 of 2, and 2 of 0 and 3: 3 is finished first, then the loop as one.
    let steps: Vec<Rc<[Step]>> = vec![
        Rc::from([(0, 1)]),
        Rc::from([(0, 2)]),
        Rc::from([(0, 0), (0, 3)]),
        Rc::from([]),
    ];
    let mut search = Search::from(0, Rc::clone(&steps[0]));
    let mut closed = Vec::new();
    while let Some(next) = search.next(|_| false) {
        match next {
            Next::Enter(value) => search.enter(value, Rc::clone(&steps[value])),
            Next::Close(mut members) => {
                members.sort_unstable();
                closed.push(members);
            }
        }
    }
    assert_eq!(closed, [vec![3], vec![0, 1, 2]]);
}

#[test]
fn a_disconnected_cycle_does_not_loop_when_collecting_reads() {
    let mut document = document();
    let mut node = document["unit"]["functions"][19]["nodes"][0].clone();
    node["id"] = json!("detached");
    node["children"] = json!(["detached"]);
    document["unit"]["functions"][19]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(node);
    with_reading(document, 19, |reading| {
        let node = reading.body.node("detached").unwrap();
        assert!(reads_within(reading, node).unwrap().is_empty());
    });
}
