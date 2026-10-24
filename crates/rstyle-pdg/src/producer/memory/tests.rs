use super::*;
use crate::{
    ByteSpan, NodeCategory,
    producer::{Operation, Transfer},
};

fn graph(nodes: &[&str], edges: &[(&str, &str, bool)]) -> Graph {
    Graph {
        nodes: nodes.iter().map(|id| operation(id)).collect(),
        transfers: edges
            .iter()
            .map(|(source, target, back)| Transfer {
                source: (*source).into(),
                target: (*target).into(),
                kind: "normal".into(),
                loop_role: back.then(|| "back_edge".into()),
            })
            .collect(),
        value_sites: Vec::new(),
    }
}

fn operation(id: &str) -> Operation {
    Operation {
        id: id.into(),
        category: NodeCategory::Other,
        span: ByteSpan { start: 0, end: 0 },
        construct_span: ByteSpan { start: 0, end: 0 },
        original_kind: "test".into(),
        role: "test".into(),
        ty: "usize".into(),
        symbol: None,
        definition: None,
        operator: None,
        literal: None,
        construct: None,
        write_form: None,
        return_form: None,
        position: None,
        operands: Vec::new(),
    }
}

fn access(id: &str, root: u32, kind: AccessKind) -> MemoryAccess {
    MemoryAccess {
        operation: id.into(),
        location: Location(root),
        kind,
    }
}

fn exit(id: &str, kind: ExitKind) -> MemoryExit {
    MemoryExit {
        operation: id.into(),
        kind,
    }
}

fn reaching(definition: Definition, loop_carried: bool) -> ReachingDefinition {
    ReachingDefinition {
        definition,
        loop_carried,
    }
}

fn entry(root: u32) -> Definition {
    Definition::Entry(Location(root))
}

fn op(id: &str) -> Definition {
    Definition::Operation(id.into())
}

fn edge(
    source: Definition,
    target: &str,
    root: u32,
    relation: (DependenceKind, bool),
) -> MemoryDependence {
    let (kind, back) = relation;
    MemoryDependence {
        source,
        target: target.into(),
        location: Location(root),
        kind,
        loop_carried: back,
    }
}

#[test]
fn straight_line_preserves_raw_war_waw_and_final_write() {
    let graph = graph(
        &["read", "first", "second", "last"],
        &[
            ("E", "read", false),
            ("read", "first", false),
            ("first", "second", false),
            ("second", "last", false),
            ("last", "X", false),
        ],
    );
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0)],
            accesses: &[
                access("read", 0, AccessKind::Load),
                access("first", 0, AccessKind::Store),
                access("second", 0, AccessKind::Store),
                access("last", 0, AccessKind::Load),
            ],
            aliases: &[],
            exits: &[exit("X", ExitKind::Normal)],
        },
    )
    .unwrap();
    let expected = BTreeSet::from([
        edge(entry(0), "read", 0, (DependenceKind::Raw, false)),
        edge(entry(0), "first", 0, (DependenceKind::Waw, false)),
        edge(op("read"), "first", 0, (DependenceKind::War, false)),
        edge(op("first"), "second", 0, (DependenceKind::Waw, false)),
        edge(op("second"), "last", 0, (DependenceKind::Raw, false)),
    ]);
    assert_eq!(
        analysis.dependencies.into_iter().collect::<BTreeSet<_>>(),
        expected
    );
    assert_eq!(
        analysis.exits["X"].reaching[&Location(0)],
        BTreeSet::from([reaching(op("second"), false)])
    );
}

#[test]
fn branch_join_keeps_untouched_entry_and_written_alternative() {
    let graph = graph(
        &["branch", "store", "load"],
        &[
            ("E", "branch", false),
            ("branch", "store", false),
            ("branch", "load", false),
            ("store", "load", false),
            ("load", "X", false),
        ],
    );
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0)],
            accesses: &[
                access("store", 0, AccessKind::Store),
                access("load", 0, AccessKind::Load),
            ],
            aliases: &[],
            exits: &[
                exit("X", ExitKind::Normal),
                exit("unreachable", ExitKind::Failure),
            ],
        },
    )
    .unwrap();
    let raw: BTreeSet<_> = analysis
        .dependencies
        .iter()
        .filter(|edge| edge.kind == DependenceKind::Raw)
        .cloned()
        .collect();
    assert_eq!(
        raw,
        BTreeSet::from([
            edge(entry(0), "load", 0, (DependenceKind::Raw, false)),
            edge(op("store"), "load", 0, (DependenceKind::Raw, false)),
        ])
    );
    assert!(!analysis.exits["unreachable"].reachable);
}

#[test]
fn loop_distinguishes_current_iteration_and_crossed_back_edge() {
    let graph = graph(
        &["guard", "load", "store"],
        &[
            ("E", "guard", false),
            ("guard", "load", false),
            ("guard", "X", false),
            ("load", "store", false),
            ("store", "guard", true),
        ],
    );
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0)],
            accesses: &[
                access("load", 0, AccessKind::Load),
                access("store", 0, AccessKind::Store),
            ],
            aliases: &[],
            exits: &[exit("X", ExitKind::Normal)],
        },
    )
    .unwrap();
    assert!(analysis.dependencies.contains(&edge(
        op("store"),
        "load",
        0,
        (DependenceKind::Raw, true)
    )));
    assert!(analysis.dependencies.contains(&edge(
        op("load"),
        "store",
        0,
        (DependenceKind::War, false)
    )));
    assert!(!analysis.dependencies.contains(&edge(
        op("store"),
        "load",
        0,
        (DependenceKind::Raw, false)
    )));
    assert_eq!(
        analysis.exits["X"].reaching[&Location(0)],
        BTreeSet::from([reaching(entry(0), false), reaching(op("store"), true),])
    );
}

#[test]
fn pending_reads_cross_back_edge_for_war_without_false_same_iteration_edge() {
    let graph = graph(
        &["store", "load"],
        &[
            ("E", "store", false),
            ("store", "load", false),
            ("load", "store", true),
            ("load", "X", false),
        ],
    );
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0)],
            accesses: &[
                access("load", 0, AccessKind::Load),
                access("store", 0, AccessKind::Store),
            ],
            aliases: &[],
            exits: &[exit("X", ExitKind::Normal)],
        },
    )
    .unwrap();
    assert!(analysis.dependencies.contains(&edge(
        op("load"),
        "store",
        0,
        (DependenceKind::War, true)
    )));
    assert!(!analysis.dependencies.contains(&edge(
        op("load"),
        "store",
        0,
        (DependenceKind::War, false)
    )));
    assert!(analysis.dependencies.contains(&edge(
        op("store"),
        "store",
        0,
        (DependenceKind::Waw, true)
    )));
}

#[test]
fn aliases_weakly_update_peers_and_preserve_cross_root_dependencies() {
    let graph = graph(
        &["read_other", "write", "read", "write_other"],
        &[
            ("E", "read_other", false),
            ("read_other", "write", false),
            ("write", "read", false),
            ("read", "write_other", false),
            ("write_other", "X", false),
        ],
    );
    let accesses = [
        access("read_other", 1, AccessKind::Load),
        access("write", 0, AccessKind::Store),
        access("read", 1, AccessKind::Load),
        access("write_other", 1, AccessKind::Store),
    ];
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0), Location(1)],
            accesses: &accesses,
            aliases: &[(Location(0), Location(1))],
            exits: &[exit("X", ExitKind::Normal)],
        },
    )
    .unwrap();
    for expected in [
        edge(op("read_other"), "write", 0, (DependenceKind::War, false)),
        edge(op("write"), "read", 1, (DependenceKind::Raw, false)),
        edge(entry(1), "read", 1, (DependenceKind::Raw, false)),
        edge(op("write"), "write_other", 1, (DependenceKind::Waw, false)),
    ] {
        assert!(analysis.dependencies.contains(&expected), "{expected:?}");
    }
    assert_eq!(
        analysis.exits["X"].reaching[&Location(0)],
        BTreeSet::from([
            reaching(op("write"), false),
            reaching(op("write_other"), false),
        ])
    );
    verify_disjoint_control(&graph, &accesses);
}

fn verify_disjoint_control(graph: &Graph, accesses: &[MemoryAccess]) {
    let disjoint = analyze(
        graph,
        AccessInventory {
            roots: &[Location(0), Location(1)],
            accesses,
            aliases: &[],
            exits: &[exit("X", ExitKind::Normal)],
        },
    )
    .unwrap();
    assert!(!disjoint.dependencies.contains(&edge(
        op("write"),
        "read",
        1,
        (DependenceKind::Raw, false)
    )));
}

#[test]
fn failure_after_prior_store_preserves_that_store_and_prevents_later_store() {
    let graph = graph(
        &["first", "check", "second"],
        &[
            ("E", "first", false),
            ("first", "check", false),
            ("check", "panic", false),
            ("check", "second", false),
            ("second", "X", false),
        ],
    );
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0), Location(1)],
            accesses: &[
                access("first", 0, AccessKind::Store),
                access("check", 1, AccessKind::Load),
                access("second", 1, AccessKind::Store),
            ],
            aliases: &[(Location(0), Location(1))],
            exits: &[
                exit("X", ExitKind::Normal),
                exit("panic", ExitKind::Failure),
            ],
        },
    )
    .unwrap();
    let failure = &analysis.exits["panic"];
    assert_eq!(failure.kind, ExitKind::Failure);
    assert_eq!(
        failure.reaching[&Location(0)],
        BTreeSet::from([reaching(op("first"), false)])
    );
    assert_eq!(
        failure.reaching[&Location(1)],
        BTreeSet::from([reaching(entry(1), false), reaching(op("first"), false),])
    );
    assert_eq!(
        analysis.exits["X"].reaching[&Location(1)],
        BTreeSet::from([reaching(op("second"), false)])
    );
}

#[test]
fn malformed_memory_identities_and_roots_are_rejected() {
    let graph = graph(&["load"], &[("E", "load", false), ("load", "X", false)]);
    let roots = [Location(0)];
    let exits = [exit("X", ExitKind::Normal)];
    for accesses in [
        vec![access("absent", 0, AccessKind::Load)],
        vec![access("load", 1, AccessKind::Load)],
        vec![
            access("load", 0, AccessKind::Load),
            access("load", 0, AccessKind::Store),
        ],
    ] {
        assert!(
            analyze(
                &graph,
                AccessInventory {
                    roots: &roots,
                    accesses: &accesses,
                    aliases: &[],
                    exits: &exits
                }
            )
            .is_err()
        );
    }
}

#[test]
fn unknown_alias_and_duplicate_root_are_rejected() {
    let graph = graph(&["load"], &[("E", "load", false), ("load", "X", false)]);
    let roots = [Location(0)];
    let exits = [exit("X", ExitKind::Normal)];
    assert!(
        analyze(
            &graph,
            AccessInventory {
                roots: &roots,
                accesses: &[],
                aliases: &[(Location(0), Location(1))],
                exits: &exits
            }
        )
        .is_err()
    );
    assert!(
        analyze(
            &graph,
            AccessInventory {
                roots: &[Location(0), Location(0)],
                accesses: &[],
                aliases: &[],
                exits: &exits
            }
        )
        .is_err()
    );
}

#[test]
fn unknown_transfers_and_continuing_exits_are_rejected() {
    let graph = graph(&["load"], &[("E", "load", false), ("load", "X", false)]);
    let roots = [Location(0)];
    let exits = [exit("X", ExitKind::Normal)];
    let mut unknown_transfer = graph.clone();
    unknown_transfer.transfers[0].target = "missing".into();
    assert!(
        analyze(
            &unknown_transfer,
            AccessInventory {
                roots: &roots,
                accesses: &[],
                aliases: &[],
                exits: &exits
            }
        )
        .is_err()
    );
    let mut continuing_exit = graph.clone();
    continuing_exit.transfers[0].source = "X".into();
    assert!(
        analyze(
            &continuing_exit,
            AccessInventory {
                roots: &roots,
                accesses: &[],
                aliases: &[],
                exits: &exits
            }
        )
        .is_err()
    );
}

#[test]
fn pairwise_possible_aliases_do_not_merge_three_formal_roots() {
    let graph = graph(
        &["store", "load"],
        &[
            ("E", "store", false),
            ("store", "load", false),
            ("load", "X", false),
        ],
    );
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0), Location(1), Location(2)],
            accesses: &[
                access("store", 0, AccessKind::Store),
                access("load", 2, AccessKind::Load),
            ],
            aliases: &[(Location(0), Location(1)), (Location(1), Location(2))],
            exits: &[exit("X", ExitKind::Normal)],
        },
    )
    .unwrap();
    assert_eq!(
        analysis.exits["X"].reaching[&Location(2)],
        BTreeSet::from([reaching(entry(2), false),])
    );
    assert!(!analysis.dependencies.contains(&edge(
        op("store"),
        "load",
        2,
        (DependenceKind::Raw, false)
    )));
}

#[test]
fn failure_on_a_later_iteration_retains_prior_writes_and_alias_alternatives() {
    let graph = graph(
        &["check", "store"],
        &[
            ("E", "check", false),
            ("check", "panic", false),
            ("check", "store", false),
            ("store", "check", true),
        ],
    );
    let analysis = analyze(
        &graph,
        AccessInventory {
            roots: &[Location(0), Location(1)],
            accesses: &[
                access("check", 1, AccessKind::Load),
                access("store", 0, AccessKind::Store),
            ],
            aliases: &[(Location(0), Location(1))],
            exits: &[exit("panic", ExitKind::Failure)],
        },
    )
    .unwrap();
    let failure = &analysis.exits["panic"];
    assert!(failure.reachable);
    assert_eq!(
        failure.reaching[&Location(0)],
        BTreeSet::from([reaching(entry(0), false), reaching(op("store"), true),])
    );
    assert_eq!(
        failure.reaching[&Location(1)],
        BTreeSet::from([
            reaching(entry(1), false),
            reaching(entry(1), true),
            reaching(op("store"), true),
        ])
    );
    assert!(analysis.dependencies.contains(&edge(
        op("store"),
        "check",
        1,
        (DependenceKind::Raw, true)
    )));
    assert!(analysis.dependencies.contains(&edge(
        op("check"),
        "store",
        0,
        (DependenceKind::War, false)
    )));
}
