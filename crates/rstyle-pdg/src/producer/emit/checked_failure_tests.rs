use super::*;
use crate::producer::Transfer;

fn graph() -> Graph {
    Graph {
        nodes: Vec::new(),
        transfers: vec![Transfer {
            source: "E".into(),
            target: "X".into(),
            kind: "normal".into(),
            loop_role: None,
        }],
        value_sites: Vec::new(),
    }
}

#[test]
fn historical_graph_keeps_normal_profile_and_edges() {
    let graph = graph();
    validate_exception_transfers(&graph).unwrap();
    assert_eq!(cfg_profile(&graph), r0_profile());
    assert!(exception_exit_ids(&graph).is_empty());
    assert_eq!(cfg_edges(&graph)[0].kind, CfgEdgeKind::Normal);
    assert_eq!(
        boundary("X", CfgNodeKind::Exit).exit_kind,
        Some(CfgExitKind::Normal)
    );
}

#[test]
fn checked_failure_emits_distinct_exception_edge_and_terminal_boundary() {
    let mut graph = graph();
    graph.transfers.push(Transfer {
        source: "checked".into(),
        target: "P:checked".into(),
        kind: "exception".into(),
        loop_role: None,
    });
    assert_eq!(cfg_edges(&graph)[1].kind, CfgEdgeKind::Exception);
    assert_eq!(
        cfg_profile(&graph).exceptional_flow,
        ExceptionalFlow::Explicit
    );
    assert_eq!(exception_exit_ids(&graph), BTreeSet::from(["P:checked"]));
    let boundary = exception_boundary("P:checked");
    assert_eq!(boundary.kind, CfgNodeKind::Exit);
    assert_eq!(boundary.exit_kind, Some(CfgExitKind::Exceptional));
    assert!(boundary.operations.is_empty());
}

#[test]
fn invalid_failure_transfers_cannot_be_silently_emitted_as_normal() {
    for (source, target, kind) in [
        ("E", "P:checked", "normal"),
        ("P:checked", "X", "normal"),
        ("E", "X", "exception"),
        ("E", "P:", "exception"),
        ("missing", "P:checked", "exception"),
    ] {
        let mut graph = graph();
        graph.transfers.push(Transfer {
            source: source.into(),
            target: target.into(),
            kind: kind.into(),
            loop_role: None,
        });
        assert!(
            validate_exception_transfers(&graph).is_err(),
            "{source}/{target}/{kind}"
        );
    }
}
