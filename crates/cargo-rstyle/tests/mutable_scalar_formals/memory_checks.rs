use rstyle_pdg::{PdgNode, producer::Pair};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const KEY: &str = "rstyle.data:mutable-memory-flow";
const LOAD: &str = "data.mutable-scalar-load";
const STORE: &str = "data.mutable-scalar-store";
type Edge = (String, String, u32, &'static str, bool);
type Alternative = (String, bool);

pub(super) fn accesses(pair: &Pair) -> Vec<&PdgNode> {
    pair.pdg
        .nodes
        .iter()
        .filter(|node| matches!(node.attributes.construct.as_deref(), Some(LOAD | STORE)))
        .collect()
}

pub(super) fn check(pair: &Pair) {
    let nodes = accesses(pair);
    assert_eq!(
        nodes
            .iter()
            .map(|node| node.attributes.position.unwrap())
            .collect::<Vec<_>>(),
        [4, 1, 1, 4, 1, 1, 4, 4]
    );
    assert_eq!(
        nodes
            .iter()
            .map(|node| node.attributes.construct.as_deref().unwrap())
            .collect::<Vec<_>>(),
        [LOAD, LOAD, LOAD, LOAD, LOAD, STORE, LOAD, STORE]
    );
    let record = &pair.pdg.extensions[KEY];
    assert_eq!(pair.cfg.extensions[KEY], *record);
    assert_eq!(record["roots"].as_array().unwrap().len(), 4);
    assert_eq!(record["aliases"].as_array().unwrap().len(), 10);
    check_dependencies(pair, &nodes);
    check_exits(pair, &nodes);
}

fn roots(pair: &Pair) -> BTreeMap<u32, &Value> {
    pair.pdg.extensions[KEY]["roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|root| (root["parameter"].as_u64().unwrap() as u32, root))
        .collect()
}

fn check_dependencies(pair: &Pair, nodes: &[&PdgNode]) {
    let roots = roots(pair);
    let indexes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("data.index"))
        .collect();
    assert_eq!(indexes.len(), 2);
    let loads = [
        nodes[0], nodes[1], nodes[2], indexes[0], nodes[3], indexes[1], nodes[4], nodes[6],
    ];
    let us = nodes[5].id.as_str();
    let cs = nodes[7].id.as_str();
    let mut expected = expected_raw(&loads, &roots, [us, cs]);
    add_ordering(&mut expected, &loads, [us, cs], &roots);
    assert_eq!(expected.len(), 46);
    let actual = actual_edges(pair, &roots);
    assert_eq!(
        pair.pdg
            .edges
            .iter()
            .filter(|edge| edge.id.starts_with("edge:mutable-memory:"))
            .count(),
        46
    );
    assert_eq!(actual, expected);
}

#[derive(Clone, Copy)]
struct ExpectedEdge<'a> {
    source: &'a str,
    target: &'a str,
    root: u32,
    kind: &'static str,
    carried: bool,
}

fn expected_raw(
    loads: &[&PdgNode],
    roots: &BTreeMap<u32, &Value>,
    stores: [&str; 2],
) -> BTreeSet<Edge> {
    let [us, cs] = stores;
    let mut expected = BTreeSet::new();
    for (load, root) in loads.iter().zip([4, 1, 1, 0, 4, 3, 1, 4]) {
        let entry = ExpectedEdge {
            source: roots[&root]["node_id"].as_str().unwrap(),
            target: &load.id,
            root,
            kind: "raw",
            carried: false,
        };
        add(&mut expected, entry);
        add_prior_writes(&mut expected, entry, [us, cs]);
    }
    add(
        &mut expected,
        ExpectedEdge {
            source: us,
            target: &loads[7].id,
            root: 4,
            kind: "raw",
            carried: false,
        },
    );

    expected
}

fn add_prior_writes(edges: &mut BTreeSet<Edge>, entry: ExpectedEdge<'_>, stores: [&str; 2]) {
    let [us, cs] = stores;
    let root = entry.root;
    add(
        edges,
        ExpectedEdge {
            source: cs,
            carried: true,
            ..entry
        },
    );
    if matches!(root, 0 | 1 | 3) {
        add(
            edges,
            ExpectedEdge {
                source: us,
                carried: true,
                ..entry
            },
        );
    }
    if matches!(root, 0 | 3) {
        add(
            edges,
            ExpectedEdge {
                carried: true,
                ..entry
            },
        );
    }
}

fn add_ordering(
    edges: &mut BTreeSet<Edge>,
    loads: &[&PdgNode],
    stores: [&str; 2],
    roots: &BTreeMap<u32, &Value>,
) {
    let [us, cs] = stores;
    for (target, root) in [(us, 1), (cs, 4)] {
        let entry = ExpectedEdge {
            source: roots[&root]["node_id"].as_str().unwrap(),
            target,
            root,
            kind: "waw",
            carried: false,
        };
        add(edges, entry);
        add(
            edges,
            ExpectedEdge {
                source: cs,
                carried: true,
                ..entry
            },
        );
        add(
            edges,
            ExpectedEdge {
                source: us,
                carried: root == 1,
                ..entry
            },
        );
    }
    add_war(edges, loads, stores);
}

fn add_war(edges: &mut BTreeSet<Edge>, loads: &[&PdgNode], stores: [&str; 2]) {
    let [us, cs] = stores;
    for load in &loads[..7] {
        add(
            edges,
            ExpectedEdge {
                source: &load.id,
                target: us,
                root: 1,
                kind: "war",
                carried: false,
            },
        );
    }
    add(
        edges,
        ExpectedEdge {
            source: &loads[7].id,
            target: us,
            root: 1,
            kind: "war",
            carried: true,
        },
    );
    for load in loads {
        add(
            edges,
            ExpectedEdge {
                source: &load.id,
                target: cs,
                root: 4,
                kind: "war",
                carried: false,
            },
        );
    }
}

fn add(edges: &mut BTreeSet<Edge>, edge: ExpectedEdge<'_>) {
    assert!(edges.insert((
        edge.source.into(),
        edge.target.into(),
        edge.root,
        edge.kind,
        edge.carried
    )));
}

fn actual_edges(pair: &Pair, roots: &BTreeMap<u32, &Value>) -> BTreeSet<Edge> {
    let locations: BTreeMap<_, _> = roots
        .iter()
        .map(|(parameter, root)| (root["location_id"].as_str().unwrap(), *parameter))
        .collect();
    pair.pdg
        .edges
        .iter()
        .filter_map(|edge| {
            let root = *locations.get(edge.location_id.as_deref()?)?;
            let kind = match edge.subkind.as_str() {
                "reaching_definition" => "raw",
                "memory_war" => "war",
                "memory_waw" => "waw",
                other => panic!("unexpected memory edge {other}"),
            };
            if kind == "war" {
                assert!(edge.definition_id.is_none());
            } else {
                let definition = pair
                    .pdg
                    .definitions
                    .iter()
                    .find(|definition| Some(&definition.id) == edge.definition_id.as_ref())
                    .unwrap();
                assert_eq!(definition.node_id, edge.source);
            }
            Some((
                edge.source.clone(),
                edge.target.clone(),
                root,
                kind,
                edge.loop_carried.unwrap(),
            ))
        })
        .collect()
}

fn check_exits(pair: &Pair, nodes: &[&PdgNode]) {
    let roots = roots(pair);
    let exits = pair.pdg.extensions[KEY]["exits"].as_array().unwrap();
    assert_eq!(exits.len(), pair.cfg.exit_node_ids.len());
    let normal = exits
        .iter()
        .find(|exit| exit["exit_kind"] == "normal")
        .unwrap();
    let normal_expected = baseline(&roots, &nodes[5].id, &nodes[7].id);
    assert_eq!(state(normal, &roots), normal_expected);
    let candidate_start = nodes[7].source_span.as_ref().unwrap().start;
    let mut before = 0;
    let mut after = 0;
    for exit in exits
        .iter()
        .filter(|exit| exit["exit_kind"] == "exceptional")
    {
        let id = exit["cfg_node_id"].as_str().unwrap();
        let edge = pair
            .cfg
            .edges
            .iter()
            .find(|edge| edge.target == id)
            .unwrap();
        assert_eq!(edge.kind, rstyle_pdg::cfg::CfgEdgeKind::Exception);
        let check = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.id == edge.source)
            .unwrap();
        assert!(exit["reachable"].as_bool().unwrap());
        if check.source_span.as_ref().unwrap().start == candidate_start {
            after += 1;
            assert_eq!(
                state(exit, &roots),
                after_first(&roots, &nodes[5].id, &nodes[7].id)
            );
        } else {
            before += 1;
            assert_eq!(state(exit, &roots), normal_expected);
        }
    }
    assert_eq!(before, 5);
    assert_eq!(after, 1);
}

fn baseline(
    roots: &BTreeMap<u32, &Value>,
    us: &str,
    cs: &str,
) -> BTreeMap<u32, BTreeSet<Alternative>> {
    roots
        .iter()
        .map(|(root, entry)| {
            let mut values = BTreeSet::from([
                (entry["node_id"].as_str().unwrap().into(), false),
                (cs.into(), true),
            ]);
            if *root != 4 {
                values.insert((us.into(), true));
            }
            if matches!(root, 0 | 3) {
                values.insert((entry["node_id"].as_str().unwrap().into(), true));
            }
            (*root, values)
        })
        .collect()
}

fn after_first(
    roots: &BTreeMap<u32, &Value>,
    us: &str,
    cs: &str,
) -> BTreeMap<u32, BTreeSet<Alternative>> {
    let mut expected = baseline(roots, us, cs);
    expected.insert(1, BTreeSet::from([(us.into(), false)]));
    for root in [0, 3, 4] {
        expected.get_mut(&root).unwrap().insert((us.into(), false));
    }
    expected
}

fn state(exit: &Value, roots: &BTreeMap<u32, &Value>) -> BTreeMap<u32, BTreeSet<Alternative>> {
    let locations: BTreeMap<_, _> = roots
        .iter()
        .map(|(root, value)| (value["location_id"].as_str().unwrap(), *root))
        .collect();
    exit["reaching"]
        .as_array()
        .unwrap()
        .iter()
        .map(|root| {
            let parameter = locations[root["location_id"].as_str().unwrap()];
            let values = root["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|definition| {
                    (
                        definition["node_id"].as_str().unwrap().into(),
                        definition["loop_carried"].as_bool().unwrap(),
                    )
                })
                .collect();
            (parameter, values)
        })
        .collect()
}
