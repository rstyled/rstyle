use super::*;
use crate::facts::{EdgeKind, PdgEdge};

fn original() -> serde_json::Value {
    let graph = crate::pdgtest::emit_loop("Cow", "value");
    serde_json::to_value(Observation {
        schema: KEY.into(),
        owner: "crate::caller".into(),
        graph,
    })
    .unwrap()
}

#[test]
fn pattern_graphs_keep_unknown_metadata_distinct_from_missing_fields() {
    let original = original();
    assert!(Observation::read(&original).is_ok());
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
        changed["graph"]["nodes"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(Observation::read(&changed).is_err(), "{field}");
    }
    let mut changed = original.clone();
    changed["graph"]["edges"][0]
        .as_object_mut()
        .unwrap()
        .remove("arg_pos");
    assert!(Observation::read(&changed).is_err());
    changed = original;
    changed["schema"] = "unregistered/2".into();
    assert!(Observation::read(&changed).is_err());
}

#[test]
fn known_empty_graph_is_available_but_absent_nodes_and_invalid_edges_are_errors() {
    let mut observation = Observation {
        schema: KEY.into(),
        owner: "crate::empty".into(),
        graph: Pdg::default(),
    };
    let empty = serde_json::to_value(&observation).unwrap();
    assert!(Observation::read(&empty).unwrap().graph.nodes.is_empty());
    let mut missing = empty;
    missing["graph"].as_object_mut().unwrap().remove("nodes");
    assert!(Observation::read(&missing).is_err());
    observation.graph.edges.push(PdgEdge {
        from: 0,
        to: 0,
        kind: EdgeKind::Data,
        arg_pos: 0,
    });
    assert!(Observation::read(&serde_json::to_value(observation).unwrap()).is_err());
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
        changed["graph"]["nodes"][4]["callee_sig"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(Observation::read(&changed).is_err(), "{field}");
    }
}
