//! Indexed lookup preserves source correspondence and malformed-input handling.
use super::*;

#[test]
fn definitions_follow_renamed_nodes_in_reversed_large_inventory() {
    let mut expected = tests::pair();
    expected.pdg.definitions = (0..4096)
        .map(|position| Definition {
            id: format!("definition-{position}"),
            node_id: format!("node-{position}"),
            symbol_id: None,
            location_id: None,
        })
        .collect();
    let mut actual = tests::pair();
    actual.pdg.definitions = expected.pdg.definitions.iter().rev().cloned().collect();
    let mut names = BTreeMap::new();
    for definition in &mut actual.pdg.definitions {
        let original = definition.node_id.clone();
        definition.node_id = format!("actual-{original}");
        definition.id = format!("actual-{}", definition.id);
        names.insert(definition.node_id.clone(), original);
    }

    add_definitions(actual.as_view(), expected.as_view(), &mut names).unwrap();
    for position in 0..4096 {
        assert_eq!(
            names[&format!("actual-definition-{position}")],
            format!("definition-{position}")
        );
    }
    names.remove("actual-node-0");
    assert_eq!(
        add_definitions(actual.as_view(), expected.as_view(), &mut names).unwrap_err(),
        ["Definition source operation mismatch"]
    );
}

#[test]
fn duplicate_definition_keys_keep_first_match_without_certifying_documents() {
    let mut expected = tests::pair();
    let first = Definition {
        id: "first-definition".into(),
        node_id: "operation".into(),
        symbol_id: None,
        location_id: None,
    };
    let mut duplicate = first.clone();
    duplicate.id = "later-definition".into();
    expected.pdg.definitions = vec![first, duplicate];
    let mut actual = tests::pair();
    actual.pdg.definitions = vec![expected.pdg.definitions[0].clone()];
    actual.pdg.definitions[0].node_id = "renamed-operation".into();
    actual.pdg.definitions[0].id = "renamed-definition".into();
    let mut names = BTreeMap::from([("renamed-operation".into(), "operation".into())]);

    add_definitions(actual.as_view(), expected.as_view(), &mut names).unwrap();
    assert_eq!(names["renamed-definition"], "first-definition");
    assert!(compare_documents(actual.as_view(), &mut expected, &names).is_err());
}

fn operations(pair: &mut Pair, entries: &[OperationMapEntry]) {
    pair.pdg.extensions.insert(
        OPERATION_MAP_KEY.into(),
        serde_json::to_value(entries).unwrap(),
    );
}

#[test]
fn cfg_correspondence_follows_reversed_large_operation_inventory() {
    let entries: Vec<_> = (0..4096)
        .map(|position| OperationMapEntry {
            pdg_node_id: format!("pdg-{position}"),
            cfg_node_id: format!("cfg-{position}"),
            cfg_operation_id: format!("operation-{position}"),
        })
        .collect();
    let mut expected = tests::pair();
    operations(&mut expected, &entries);
    let mut actual = tests::pair();
    let mut names = BTreeMap::new();
    let reversed: Vec<_> = entries
        .iter()
        .rev()
        .map(|entry| {
            names.insert(
                format!("actual-{}", entry.pdg_node_id),
                entry.pdg_node_id.clone(),
            );
            OperationMapEntry {
                pdg_node_id: format!("actual-{}", entry.pdg_node_id),
                cfg_node_id: format!("actual-{}", entry.cfg_node_id),
                cfg_operation_id: format!("actual-{}", entry.cfg_operation_id),
            }
        })
        .collect();
    operations(&mut actual, &reversed);

    add_cfg_names(actual.as_view(), expected.as_view(), &mut names).unwrap();
    for position in 0..4096 {
        assert_eq!(
            names[&format!("actual-operation-{position}")],
            format!("operation-{position}")
        );
        assert_eq!(
            names[&format!("actual-cfg-{position}")],
            format!("cfg-{position}")
        );
    }
}

#[test]
fn cfg_correspondence_preserves_first_match_and_rejects_unmapped_nodes() {
    let mut expected = tests::pair();
    let first = OperationMapEntry {
        pdg_node_id: "operation".into(),
        cfg_node_id: "cfg-first".into(),
        cfg_operation_id: "cfg-operation-first".into(),
    };
    let mut duplicate = first.clone();
    duplicate.cfg_node_id = "cfg-later".into();
    operations(&mut expected, &[first.clone(), duplicate]);
    let mut actual = tests::pair();
    operations(
        &mut actual,
        &[OperationMapEntry {
            pdg_node_id: "actual-operation".into(),
            cfg_node_id: "actual-cfg".into(),
            cfg_operation_id: "actual-cfg-operation".into(),
        }],
    );
    let mut names = BTreeMap::from([("actual-operation".into(), "operation".into())]);

    add_cfg_names(actual.as_view(), expected.as_view(), &mut names).unwrap();
    assert_eq!(names["actual-cfg"], first.cfg_node_id);
    assert_eq!(names["actual-cfg-operation"], first.cfg_operation_id);
    assert!(compare_documents(actual.as_view(), &mut expected, &names).is_err());
    names.remove("actual-operation");
    assert_eq!(
        add_cfg_names(actual.as_view(), expected.as_view(), &mut names).unwrap_err(),
        ["CFG operation correspondence mismatch"]
    );
}
