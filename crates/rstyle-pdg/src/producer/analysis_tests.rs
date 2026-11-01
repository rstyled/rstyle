use super::*;

fn baseline_cyclic(vertices: &BTreeSet<String>, successors: &Sets) -> Sets {
    let mut result: Sets = vertices
        .iter()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for candidate in vertices {
        let mut must_encounter = BTreeSet::from([candidate.clone()]);
        loop {
            let next: BTreeSet<_> = must_encounter
                .iter()
                .cloned()
                .chain(
                    successors
                        .iter()
                        .filter(|(_, next)| !next.is_empty() && next.is_subset(&must_encounter))
                        .map(|(id, _)| id.clone()),
                )
                .collect();
            if next == must_encounter {
                break;
            }
            must_encounter = next;
        }
        for vertex in must_encounter {
            result
                .get_mut(&vertex)
                .expect("reachable vertex")
                .insert(candidate.clone());
        }
    }
    result
}

fn vertices(n: usize) -> BTreeSet<String> {
    (0..n).map(|i| format!("v{i}")).collect()
}

fn graph_from(successors: &Sets) -> Graph {
    let transfers = successors
        .iter()
        .flat_map(|(source, targets)| {
            targets.iter().flat_map(move |target| {
                ["true", "false"].map(|kind| super::super::Transfer {
                    source: source.clone(),
                    target: target.clone(),
                    kind: kind.into(),
                    loop_role: None,
                })
            })
        })
        .collect();

    Graph {
        nodes: Vec::new(),
        transfers,
        value_sites: Vec::new(),
    }
}

// Independent oracle: deleting z may create sinks, but those are not maximal
// paths in the original graph. A repeated vertex witnesses an infinite path.
fn avoids(id: &str, candidate: &str, successors: &Sets, path: &mut BTreeSet<String>) -> bool {
    if id == candidate {
        return false;
    }
    if successors[id].is_empty() || path.contains(id) {
        return true;
    }
    path.insert(id.into());
    let result = successors[id]
        .iter()
        .any(|next| avoids(next, candidate, successors, path));
    path.remove(id);

    result
}

fn oracle_cyclic(vertices: &BTreeSet<String>, successors: &Sets) -> Sets {
    vertices
        .iter()
        .map(|id| {
            let post = vertices
                .iter()
                .filter(|z| !avoids(id, z, successors, &mut BTreeSet::new()))
                .cloned()
                .collect();
            (id.clone(), post)
        })
        .collect()
}

fn compare_cyclic(successors: &Sets) {
    let vertices = successors.keys().cloned().collect();
    let baseline = baseline_cyclic(&vertices, successors);
    let actual = divergence_postdominators(&vertices, successors);
    assert_eq!(actual, baseline, "{successors:?}");
    assert_eq!(
        actual,
        oracle_cyclic(&vertices, successors),
        "{successors:?}"
    );
    let graph = graph_from(successors);

    assert_eq!(
        cyclic_control_edges(&graph, &vertices, &actual),
        cyclic_control_edges(&graph, &vertices, &baseline)
    );
}

#[test]
fn exhaustive_cyclic() {
    // Every labelled directed graph, including loops and disconnected vertices.
    // 66,067 graphs through four vertices, all candidates and both outcomes.
    for n in 0..=4 {
        let ids: Vec<_> = vertices(n).into_iter().collect();
        for mask in 0..(1u64 << (n * n)) {
            let successors = ids
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    let targets = ids
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| mask & (1 << (i * n + j)) != 0)
                        .map(|(_, id)| id.clone())
                        .collect();
                    (id.clone(), targets)
                })
                .collect();
            compare_cyclic(&successors);
        }
    }
}

fn fixture(edges: &[(&str, &str)]) -> Sets {
    let ids: BTreeSet<_> = edges
        .iter()
        .flat_map(|(a, b)| [a.to_string(), b.to_string()])
        .collect();
    ids.into_iter()
        .map(|id| {
            let next = edges
                .iter()
                .filter(|(a, _)| *a == id)
                .map(|(_, b)| b.to_string())
                .collect();
            (id, next)
        })
        .collect()
}

#[test]
fn cyclic_boundaries_and_lowered_loop() {
    // Original sinks, candidate-created sinks, self loops, mutual unavoidable
    // cycles, avoiding cycles, duplicate outcome transfers, E/X suppression.
    for edges in [
        vec![("E", "a"), ("a", "X")],
        vec![("E", "a"), ("a", "a"), ("a", "X")],
        vec![("E", "a"), ("a", "b"), ("b", "a")],
        vec![("E", "a"), ("a", "b"), ("b", "b"), ("a", "X")],
        vec![
            ("E", "n2"),
            ("n2", "n3"),
            ("n3", "n4"),
            ("n3", "n6"),
            ("n4", "n5"),
            ("n5", "n2"),
            ("n6", "n7"),
            ("n7", "X"),
        ],
    ] {
        compare_cyclic(&fixture(&edges));
    }
}

fn family(kind: &str, n: usize) -> Sets {
    let ids: Vec<_> = vertices(n).into_iter().collect();
    ids.iter()
        .enumerate()
        .map(|(i, id)| {
            let targets = match kind {
                "cycle" => vec![ids[(i + 1) % n].clone()],
                "avoiding" if i + 1 < n => vec![id.clone(), ids[i + 1].clone()],
                "branch" if i + 2 < n => vec![ids[i + 1].clone(), ids[i + 2].clone()],
                _ if i + 1 < n => vec![ids[i + 1].clone()],
                _ => Vec::new(),
            };
            (id.clone(), targets.into_iter().collect())
        })
        .collect()
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, u128) {
    let start = std::time::Instant::now();
    let result = std::hint::black_box(f());
    (result, start.elapsed().as_nanos())
}

#[test]
#[ignore = "release benchmark; run with --ignored --nocapture"]
fn benchmark_cyclic() {
    for row in cyclic_samples() {
        println!("{row}");
    }
}

fn cyclic_samples() -> Vec<String> {
    let mut samples = Vec::new();
    for kind in ["chain", "branch", "cycle", "avoiding"] {
        for n in [32, 64, 128] {
            samples.extend(benchmark_cyclic_graph(kind, &family(kind, n)));
        }
    }
    samples.extend(benchmark_cyclic_graph(
        "lowered-while",
        &fixture(&[
            ("E", "n2"),
            ("n2", "n3"),
            ("n3", "n4"),
            ("n3", "n6"),
            ("n4", "n5"),
            ("n5", "n2"),
            ("n6", "n7"),
            ("n7", "X"),
        ]),
    ));

    samples
}

fn benchmark_cyclic_graph(kind: &str, successors: &Sets) -> Vec<String> {
    let vertices = successors.keys().cloned().collect();
    let graph = graph_from(successors);
    let old_scans = baseline_scans(&vertices, successors);
    (0..3).map(|repeat| {
        let (old, old_ns) = timed(|| baseline_cyclic(&vertices, successors));
        let (new, new_ns) = timed(|| divergence_postdominators(&vertices, successors));
        assert_eq!(old, new);
        let (indexed, index_ns) = timed(|| Topology::new(&vertices, successors));
        let (relation, propagation_ns) = timed(|| indexed.divergence_relation());
        let (materialized, representation_ns) = timed(|| indexed.materialize(&relation));
        assert_eq!(new, materialized);
        let (edges, output_ns) = timed(|| cyclic_control_edges(&graph, &vertices, &new));
        let (_, old_complete_ns) = timed(|| {
            let post = baseline_cyclic(&vertices, successors);
            cyclic_control_edges(&graph, &vertices, &post)
        });
        let (_, complete_ns) = timed(|| {
            let post = divergence_postdominators(&vertices, successors);
            cyclic_control_edges(&graph, &vertices, &post)
        });
        let visits: usize = relation.iter().zip(&indexed.predecessors).map(|(zs, ps)| zs.len()*ps.len()).sum();

        format!("cyclic,{kind},{},{},{},{},{repeat},{old_ns},{new_ns},{index_ns},{propagation_ns},{representation_ns},{output_ns},{old_complete_ns},{complete_ns},{old_scans},{visits}", successors.len(), successors.values().map(BTreeSet::len).sum::<usize>(), new.values().map(BTreeSet::len).sum::<usize>(), edges.len())
    }).collect()
}

// Counts whole-graph vertex predicate evaluations, including the final round.
fn baseline_scans(vertices: &BTreeSet<String>, successors: &Sets) -> usize {
    vertices
        .iter()
        .map(|candidate| {
            let mut admitted = BTreeSet::from([candidate.clone()]);
            let mut scans = 0;
            loop {
                scans += successors.len();
                let next: BTreeSet<_> = admitted
                    .iter()
                    .cloned()
                    .chain(
                        successors
                            .iter()
                            .filter(|(_, next)| !next.is_empty() && next.is_subset(&admitted))
                            .map(|(id, _)| id.clone()),
                    )
                    .collect();
                if next == admitted {
                    return scans;
                }
                admitted = next;
            }
        })
        .sum()
}

#[test]
fn captured_lowered_baseline() {
    let successors = lowered_while();
    let vertices = successors.keys().cloned().collect();
    let baseline = baseline_cyclic(&vertices, &successors);
    let control = cyclic_control_edges(&graph_from(&successors), &vertices, &baseline);

    print_baseline(&baseline, &control);
}

fn print_baseline(baseline: &Sets, control: &[Dependence]) {
    println!("relation={}", serde_json::to_string(baseline).unwrap());
    println!("control={}", serde_json::to_string(control).unwrap());
}

fn lowered_while() -> Sets {
    fixture(&[
        ("E", "n2"),
        ("n2", "n3"),
        ("n3", "n4"),
        ("n3", "n6"),
        ("n4", "n5"),
        ("n5", "n2"),
        ("n6", "n7"),
        ("n7", "X"),
    ])
}

fn dag(n: usize, mask: u64) -> Sets {
    let ids: Vec<_> = (0..n)
        .map(|i| {
            if i + 1 == n {
                "X".into()
            } else if i == 0 {
                "E".into()
            } else {
                format!("n{i}")
            }
        })
        .collect();
    let mut bit = 0;
    ids.iter()
        .enumerate()
        .map(|(i, id)| {
            let targets = ids
                .iter()
                .skip(i + 1)
                .filter(|_| {
                    let present = mask & (1 << bit) != 0;
                    bit += 1;
                    present
                })
                .cloned()
                .collect();
            (id.clone(), targets)
        })
        .collect()
}

fn exit_paths(id: &str, successors: &Sets) -> Vec<BTreeSet<String>> {
    if id == "X" {
        return vec![BTreeSet::from(["X".into()])];
    }
    successors[id]
        .iter()
        .flat_map(|next| {
            exit_paths(next, successors).into_iter().map(|mut path| {
                path.insert(id.into());
                path
            })
        })
        .collect()
}

fn oracle_dag(successors: &Sets) -> Sets {
    successors
        .keys()
        .map(|id| {
            let paths = exit_paths(id, successors);
            let common = paths.iter().skip(1).fold(paths[0].clone(), |shared, path| {
                shared.intersection(path).cloned().collect()
            });
            (id.clone(), common)
        })
        .collect()
}

fn reconstruct(tree: &PostTree) -> Sets {
    tree.ids
        .iter()
        .map(|id| {
            let post = tree
                .ids
                .iter()
                .filter(|z| tree.contains(id, z))
                .map(|id| (*id).clone())
                .collect();
            ((*id).clone(), post)
        })
        .collect()
}

fn compare_dag(successors: &Sets) {
    let vertices: BTreeSet<_> = successors.keys().cloned().collect();
    let baseline = dominators(&vertices, successors, "X");
    let tree = acyclic_postdominators(&vertices, successors);
    let actual = reconstruct(&tree);
    assert_eq!(actual, baseline, "{successors:?}");
    assert_eq!(actual, oracle_dag(successors), "{successors:?}");
    for id in &vertices {
        assert_eq!(tree.immediate(id), immediate(id, &baseline));
    }
    let graph = graph_from(successors);

    assert_eq!(
        control_edges(&graph, &vertices, &tree),
        baseline_control_edges(&graph, &vertices, &baseline)
    );
}

#[test]
fn exhaustive_dags() {
    // Every forward-edge DAG through six vertices in one fixed topological
    // labelling (33,867 graphs); relabelling cannot change the relation.
    let mut valid = 0;
    for n in 1..=6 {
        for mask in 0..(1 << (n * (n - 1) / 2)) {
            let successors = dag(n, mask);
            let vertices = successors.keys().cloned().collect();
            let accepted = check_acyclic_exit(&graph_from(&successors), &vertices).is_ok();
            assert_eq!(
                accepted,
                successors
                    .iter()
                    .all(|(id, next)| id == "X" || !next.is_empty())
            );
            if accepted {
                compare_dag(&successors);
                valid += 1;
            }
        }
    }
    assert_eq!(valid, 10106);
}

#[test]
fn ordinary_boundaries_and_rejection() {
    for edges in [
        vec![("E", "X")],
        vec![("E", "a"), ("a", "b"), ("a", "c"), ("b", "X"), ("c", "X")],
        // Existing root semantics: X outgoing edges are ignored for postdom.
        vec![("E", "a"), ("a", "X"), ("X", "a")],
    ] {
        compare_dag(&fixture(&edges));
    }
    for edges in [
        vec![("E", "a")],
        vec![("E", "a"), ("a", "a"), ("a", "X")],
        vec![("E", "a"), ("a", "b"), ("b", "a"), ("E", "X")],
    ] {
        assert!(analyze(&graph_from(&fixture(&edges))).is_err());
    }
}

fn dag_family(kind: &str, n: usize) -> Sets {
    let successors = family(kind, n);
    let sink = successors
        .iter()
        .find(|(_, next)| next.is_empty())
        .unwrap()
        .0;
    successors
        .iter()
        .map(|(id, next)| {
            let rename = |id: &String| {
                if id == sink {
                    "X".to_string()
                } else {
                    id.clone()
                }
            };
            (rename(id), next.iter().map(rename).collect())
        })
        .collect()
}

#[test]
#[ignore = "release benchmark; run with --ignored --nocapture"]
fn benchmark_dag() {
    for row in dag_samples() {
        println!("{row}");
    }
}

fn dag_samples() -> Vec<String> {
    let mut rows = Vec::new();
    for kind in ["chain", "branch"] {
        for n in [32, 64, 128] {
            rows.extend(benchmark_dag_graph(kind, &dag_family(kind, n)));
        }
    }
    let case = lowered_cases().remove(0);
    let successors = neighbours(&case.graph, &reachability(&case.graph), true);
    rows.extend(benchmark_dag_graph("lowered-if", &successors));

    rows
}

fn benchmark_dag_graph(kind: &str, successors: &Sets) -> Vec<String> {
    let vertices = successors.keys().cloned().collect();
    let graph = graph_from(successors);
    let scans = ordinary_scans(&vertices, successors);
    (0..3).map(|repeat| {
        let (old, old_ns) = timed(|| dominators(&vertices, successors, "X"));
        let (new, new_ns) = timed(|| acyclic_postdominators(&vertices, successors));
        assert_eq!(old, reconstruct(&new));
        let (indexed, index_ns) = timed(|| Topology::new(&vertices, successors));
        let (order, schedule_ns) = timed(|| indexed.exit_order());
        let (mut tree, allocation_ns) = timed(|| PostTree::empty(&indexed, order[0]));
        let (_, evaluation_ns) = timed(|| tree.attach_order(&indexed, &order));
        let (_, reconstruction_ns) = timed(|| reconstruct(&tree));
        let (edges, output_ns) = timed(|| control_edges(&graph, &vertices, &new));
        let (_, old_complete_ns) = timed(|| baseline_control_edges(&graph, &vertices, &dominators(&vertices, successors, "X")));
        let (_, new_complete_ns) = timed(|| control_edges(&graph, &vertices, &acyclic_postdominators(&vertices, successors)));

        let lca_calls: usize = indexed.successors.iter().enumerate()
            .filter(|(vertex, _)| indexed.ids[*vertex] != "X")
            .map(|(_, next)| next.len().saturating_sub(1)).sum();
        let jump_cells = (order.len()-1) * tree.jumps[0].len();
        let predecessor_visits: usize = indexed.predecessors.iter().map(Vec::len).sum();

        format!("dag,{kind},{},{},{},{},{repeat},{old_ns},{new_ns},{index_ns},{schedule_ns},{allocation_ns},{evaluation_ns},{reconstruction_ns},{output_ns},{old_complete_ns},{new_complete_ns},{scans},{},{predecessor_visits},{lca_calls},{jump_cells}", successors.len(), successors.values().map(BTreeSet::len).sum::<usize>(), old.values().map(BTreeSet::len).sum::<usize>(), edges.len(), order.len())
    }).collect()
}

fn ordinary_scans(vertices: &BTreeSet<String>, successors: &Sets) -> usize {
    let mut sets: Sets = vertices
        .iter()
        .map(|id| {
            (
                id.clone(),
                if id == "X" {
                    BTreeSet::from([id.clone()])
                } else {
                    vertices.clone()
                },
            )
        })
        .collect();
    let mut scans = 0;
    loop {
        scans += vertices.len();
        let next: Sets = vertices
            .iter()
            .map(|id| {
                let post = if id == "X" {
                    BTreeSet::from([id.clone()])
                } else {
                    successors[id]
                        .iter()
                        .fold(vertices.clone(), |shared, next| {
                            shared.intersection(&sets[next]).cloned().collect()
                        })
                        .into_iter()
                        .chain([id.clone()])
                        .collect()
                };
                (id.clone(), post)
            })
            .collect();
        if next == sets {
            return scans;
        }
        sets = next;
    }
}

fn baseline_control_edges(
    graph: &Graph,
    reached: &BTreeSet<String>,
    post: &Sets,
) -> Vec<Dependence> {
    graph
        .transfers
        .iter()
        .filter(|e| {
            reached.contains(&e.source)
                && matches!(e.kind.as_str(), "true" | "false")
                && !post[&e.source].contains(&e.target)
        })
        .flat_map(|transfer| {
            let stop = immediate(&transfer.source, post);
            let mut current = Some(transfer.target.clone());
            let mut edges = Vec::new();
            while current.is_some() && current != stop {
                let id = current.expect("current postdominator");
                if id != "X" {
                    edges.push(Dependence {
                        outcome: Some(transfer.kind.clone()),
                        ..edge(&transfer.source, &id, EdgeFamily::Control, "postdominance")
                    });
                }
                current = immediate(&id, post);
            }
            edges
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct LoweredCase {
    cyclic: bool,
    graph: Graph,
    edges: Vec<Dependence>,
}

fn lowered_cases() -> Vec<LoweredCase> {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/postdominance_lowered.json"
    ))
    .unwrap()
}

#[test]
fn lowered_cfgs_preserve_every_dependence() {
    for case in lowered_cases() {
        let baseline = baseline_analysis(&case.graph, case.cyclic).unwrap();
        let actual = analyze_policy(&case.graph, case.cyclic).unwrap();
        assert_eq!(baseline.edges, case.edges);
        assert_eq!(actual, baseline);
    }
}

fn baseline_analysis(graph: &Graph, cyclic: bool) -> Result<Analysis, String> {
    baseline_analysis_using(graph, cyclic, divergence_postdominators)
}

fn baseline_analysis_using(
    graph: &Graph,
    cyclic: bool,
    cyclic_post: fn(&BTreeSet<String>, &Sets) -> Sets,
) -> Result<Analysis, String> {
    let reachable = reachability(graph);
    if !cyclic {
        check_acyclic_exit(graph, &reachable)?;
    }
    let predecessors = neighbours(graph, &reachable, false);
    let successors = neighbours(graph, &reachable, true);
    let dom = dominators(&reachable, &predecessors, "E");
    let post = if cyclic {
        cyclic_post(&reachable, &successors)
    } else {
        dominators(&reachable, &successors, "X")
    };
    let (definition_in, definition_out) = reaching(graph, &reachable, &predecessors);
    let edges = data_edges(graph, &reachable, &definition_in)?;
    let execution = execution_edges(graph, &reachable, &dom);
    let control = if cyclic {
        cyclic_control_edges(graph, &reachable, &post)
    } else {
        baseline_control_edges(graph, &reachable, &post)
    };

    Ok(Analysis {
        edges: edges.into_iter().chain(execution).chain(control).collect(),
        reachable,
        definition_in,
        definition_out,
    })
}

#[test]
#[ignore = "release benchmark including unchanged analysis costs"]
fn benchmark_lowered_complete() {
    for row in lowered_complete_samples() {
        println!("{row}");
    }
}

fn lowered_complete_samples() -> Vec<String> {
    lowered_cases()
        .iter()
        .flat_map(|case| {
            (0..3)
                .map(|repeat| {
                    let (old, old_ns) =
                        timed(|| baseline_analysis(&case.graph, case.cyclic).unwrap());
                    let (new, new_ns) = timed(|| analyze_policy(&case.graph, case.cyclic).unwrap());
                    assert_eq!(old, new);

                    format!(
                        "lowered-complete,{},{},{},{repeat},{old_ns},{new_ns}",
                        case.cyclic,
                        new.reachable.len(),
                        new.edges.len()
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn deep_tree_lifting() {
    let mut successors = dag_family("chain", 80);
    let ids: Vec<_> = family("chain", 80).keys().cloned().collect();
    successors
        .get_mut(&ids[0])
        .unwrap()
        .extend([ids[32].clone(), ids[63].clone()]);
    successors
        .get_mut(&ids[20])
        .unwrap()
        .insert(ids[50].clone());

    compare_dag(&successors);
}

#[test]
fn unreachable_cycles_remain_ignored() {
    let graph = graph_from(&fixture(&[("E", "X"), ("unreachable", "unreachable")]));
    assert_eq!(
        analyze(&graph).unwrap(),
        baseline_analysis(&graph, false).unwrap()
    );
}

fn lexical_operation(id: &str, definition: Option<&str>, category: NodeCategory) -> Operation {
    Operation {
        id: id.into(),
        symbol: Some("s".into()),
        definition: definition.map(str::to_string),
        category,
        ..lowered_cases().remove(0).graph.nodes.remove(0)
    }
}

#[test]
fn initial_entry_seed_reaches_cycles_before_entry_definition_replaces_it() {
    for edges in [
        vec![("E", "a"), ("a", "a"), ("a", "X")],
        vec![("E", "a"), ("a", "b"), ("b", "b"), ("b", "X")],
    ] {
        let mut graph = graph_from(&fixture(&edges));
        graph.nodes = vec![
            lexical_operation("unreachable", Some("A"), NodeCategory::FormalInput),
            lexical_operation("E", Some("B"), NodeCategory::Assignment),
        ];
        let actual = analyze_cyclic(&graph).unwrap();
        assert_eq!(actual, baseline_analysis(&graph, true).unwrap());
        assert_eq!(actual.definition_in["E"]["s"], BTreeSet::from(["A".into()]));
        assert_eq!(
            actual.definition_out["E"]["s"],
            BTreeSet::from(["B".into()])
        );
        assert_eq!(
            actual.definition_in["X"]["s"],
            BTreeSet::from(["A".into(), "B".into()])
        );
    }
}

#[test]
fn duplicate_operations_and_definitions_keep_first_source_and_node_order() {
    let mut graph = graph_from(&fixture(&[("E", "w"), ("w", "r"), ("r", "X")]));
    graph.nodes = vec![
        lexical_operation("dead", Some("A"), NodeCategory::FormalInput),
        lexical_operation("dead2", Some("B"), NodeCategory::FormalInput),
        lexical_operation("source", Some("C"), NodeCategory::Assignment),
        lexical_operation("w", Some("C"), NodeCategory::Assignment),
        lexical_operation("w", Some("D"), NodeCategory::Assignment),
        lexical_operation("r", None, NodeCategory::Read),
        lexical_operation("r", None, NodeCategory::Read),
    ];
    let actual = analyze(&graph).unwrap();
    assert_eq!(actual, baseline_analysis(&graph, false).unwrap());
    assert_eq!(actual.definition_in["E"]["s"], BTreeSet::from(["B".into()]));
    assert_eq!(
        actual.definition_out["w"]["s"],
        BTreeSet::from(["C".into()])
    );
    let reads: Vec<_> = actual
        .edges
        .iter()
        .filter(|edge| edge.subkind == "reaching_definition")
        .collect();
    assert_eq!(reads.len(), 2);
    assert!(
        reads
            .iter()
            .all(|edge| edge.source == "source" && edge.target == "r")
    );
}

#[test]
#[ignore = "supplemental full-analyzer comparison for commit 1"]
fn benchmark_original_cyclic_complete() {
    for row in original_cyclic_samples() {
        println!("{row}");
    }
}

fn original_cyclic_samples() -> Vec<String> {
    let case = lowered_cases().remove(1);
    (0..3)
        .map(|repeat| {
            let (old, old_ns) =
                timed(|| baseline_analysis_using(&case.graph, true, baseline_cyclic).unwrap());
            let (new, new_ns) = timed(|| baseline_analysis(&case.graph, true).unwrap());
            assert_eq!(old, new);

            format!(
                "lowered-original-cyclic,{},{},{repeat},{old_ns},{new_ns}",
                new.reachable.len(),
                new.edges.len()
            )
        })
        .collect()
}
