use super::*;
use crate::facts::{EdgeKind, PdgEdge};

fn original() -> serde_json::Value {
    serde_json::to_value(crate::pdgtest::emit_loop("Cow", "value")).unwrap()
}

#[test]
fn pattern_graphs_keep_unknown_metadata_distinct_from_missing_fields() {
    let original = original();
    assert!(read_graph(&original).is_ok());
    for field in [
        "ty",
        "callee_id",
        "callee_sig",
        "macro_name",
        "lit_kind",
        "detail",
        "line",
    ] {
        let mut changed = original.clone();
        changed["nodes"][0].as_object_mut().unwrap().remove(field);
        assert!(read_graph(&changed).is_err(), "{field}");
    }
    let mut changed = original;
    changed["edges"][0]
        .as_object_mut()
        .unwrap()
        .remove("arg_pos");
    assert!(read_graph(&changed).is_err());
}

#[test]
fn known_empty_graph_is_available_but_absent_nodes_and_invalid_edges_are_errors() {
    let mut graph = Pdg::default();
    let empty = serde_json::to_value(&graph).unwrap();
    assert!(read_graph(&empty).unwrap().nodes.is_empty());
    let mut missing = empty;
    missing.as_object_mut().unwrap().remove("nodes");
    assert!(read_graph(&missing).is_err());
    graph.edges.push(PdgEdge {
        from: 0,
        to: 0,
        kind: EdgeKind::Data,
        arg_pos: 0,
    });
    assert!(read_graph(&serde_json::to_value(graph).unwrap()).is_err());
}

#[test]
fn missing_signature_metadata_cannot_default_to_nongeneric_safe_calls() {
    let original = original();
    for field in [
        "self_ty",
        "self_args",
        "params",
        "is_async",
        "is_unsafe",
        "is_const",
        "generics",
    ] {
        let mut changed = original.clone();
        changed["nodes"][4]["callee_sig"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(read_graph(&changed).is_err(), "{field}");
    }
}
