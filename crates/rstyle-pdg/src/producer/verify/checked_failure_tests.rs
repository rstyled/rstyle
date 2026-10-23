use super::*;

fn cfg() -> CfgDocument {
    let mut cfg: CfgDocument = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor-compat/cfg.example.json"
    )))
    .unwrap();
    add_failure(&mut cfg, "panic-a", "n1");
    add_failure(&mut cfg, "panic-b", "n2");
    cfg
}

fn add_failure(cfg: &mut CfgDocument, exit: &str, source: &str) {
    cfg.exit_node_ids.push(exit.into());
    cfg.nodes.push(CfgNode {
        id: exit.into(),
        kind: CfgNodeKind::Exit,
        origin: "checked failure".into(),
        exit_kind: Some(CfgExitKind::Exceptional),
        operations: Vec::new(),
    });
    cfg.edges.push(CfgEdge {
        id: format!("fail-{exit}"),
        source: source.into(),
        target: exit.into(),
        kind: CfgEdgeKind::Exception,
        case_value: None,
        loop_role: None,
        reason: None,
    });
}

#[test]
fn exception_exit_correspondence_uses_renamed_checked_sources() {
    let expected = cfg();
    let mut actual = cfg();
    actual.exit_node_ids[1] = "renamed-panic".into();
    actual
        .nodes
        .iter_mut()
        .find(|node| node.id == "panic-a")
        .unwrap()
        .id = "renamed-panic".into();
    let edge = actual
        .edges
        .iter_mut()
        .find(|edge| edge.target == "panic-a")
        .unwrap();
    edge.target = "renamed-panic".into();
    edge.source = "renamed-check".into();
    let mut names = BTreeMap::from([("renamed-check".into(), "n1".into())]);
    add_exit_names(&actual, &expected, &mut names).unwrap();
    assert_eq!(names["renamed-panic"], "panic-a");
    assert_eq!(names["panic-b"], "panic-b");
}

#[test]
fn missing_merged_reclassified_and_wrong_source_exits_fail() {
    for mutation in ["missing", "merged", "kind", "edge", "source", "duplicate"] {
        let mut actual = cfg();
        match mutation {
            "missing" => {
                actual.exit_node_ids.pop();
            }
            "merged" => {
                actual.exit_node_ids[2] = "panic-a".into();
            }
            "kind" => {
                actual
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == "panic-a")
                    .unwrap()
                    .exit_kind = Some(CfgExitKind::Normal);
            }
            "edge" => {
                actual.edges.last_mut().unwrap().kind = CfgEdgeKind::Normal;
            }
            "source" => {
                actual.edges.last_mut().unwrap().source = "n4".into();
            }
            "duplicate" => {
                actual.edges.push(actual.edges.last().unwrap().clone());
            }
            _ => unreachable!(),
        }
        assert!(
            add_exit_names(&actual, &cfg(), &mut BTreeMap::new()).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn one_historical_exit_keeps_existing_name_correspondence() {
    let mut expected = cfg();
    expected.exit_node_ids.truncate(1);
    let mut actual = expected.clone();
    actual.exit_node_ids[0] = "renamed-normal".into();
    let mut names = BTreeMap::new();
    add_exit_names(&actual, &expected, &mut names).unwrap();
    assert_eq!(names["renamed-normal"], "n7");
}
