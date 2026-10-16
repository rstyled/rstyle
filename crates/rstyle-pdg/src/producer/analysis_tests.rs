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
        let (indexed, index_ns) = timed(|| PostGraph::new(&vertices, successors));
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
