use super::*;

#[test]
fn finite_must_evaluation_matches_all_simple_path_intersections() {
    for repeated_vertex in [false, true] {
        let states: Vec<_> = (0..3)
            .map(|index| State {
                vertex: format!("c{}", if repeated_vertex { index % 2 } else { index }),
                outcomes: BTreeMap::new(),
            })
            .collect();
        for mask in 0..64u16 {
            let candidates: Vec<_> = (0..3)
                .flat_map(|source| (1..3).map(move |target| (source, target)))
                .collect();
            let edges: BTreeSet<_> = candidates
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, edge)| *edge)
                .collect();
            let actual = must_evaluate(&states, &edges);
            for (target, actual) in actual.iter().enumerate() {
                if let Some(expected) = path_intersection(target, &states, &edges) {
                    assert_eq!(
                        *actual, expected,
                        "mask={mask} target={target} repeated={repeated_vertex}"
                    );
                }
            }
        }
    }
}

// Independent finite-path oracle for small graphs. Removing a repeated state
// cannot remove a must-visit requirement, so simple paths suffice here.
fn path_intersection(
    target: usize,
    states: &[State],
    edges: &BTreeSet<(usize, usize)>,
) -> Option<BTreeSet<String>> {
    let mut pending = vec![vec![0]];
    let mut result: Option<BTreeSet<String>> = None;
    while let Some(path) = pending.pop() {
        let end = *path.last().unwrap();
        if end == target {
            let visited: BTreeSet<_> = path[..path.len() - 1]
                .iter()
                .map(|index| states[*index].vertex.clone())
                .collect();
            result = Some(match result {
                Some(prior) => prior.intersection(&visited).cloned().collect(),
                None => visited,
            });
            continue;
        }
        for (_, next) in edges.iter().filter(|(source, _)| *source == end) {
            if !path.contains(next) {
                pending.push(path.iter().copied().chain(std::iter::once(*next)).collect());
            }
        }
    }
    result
}

#[test]
fn reset_aware_must_evaluation_matches_finite_execution_states() {
    let states: Vec<_> = (0..3)
        .map(|index| State {
            vertex: format!("c{index}"),
            outcomes: BTreeMap::new(),
        })
        .collect();
    let candidates: Vec<_> = (0..3)
        .flat_map(|source| (1..3).map(move |target| (source, target)))
        .collect();
    for mask in 0..64u16 {
        let edges: BTreeSet<_> = candidates
            .iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, edge)| *edge)
            .collect();
        // Network::build only interns reachable states. Exercise that same
        // precondition, established independently by exhaustive exploration.
        if execution_intersections(&edges, 0)
            .iter()
            .any(BTreeSet::is_empty)
        {
            continue;
        }
        for killed in 0..8u8 {
            let region: BTreeSet<_> = (0..3)
                .filter(|bit| killed & (1 << bit) != 0)
                .map(|index| format!("c{index}"))
                .collect();
            let resets = Resets(
                edges
                    .iter()
                    .filter(|(source, target)| source >= target)
                    .map(|(source, target)| {
                        ((format!("c{source}"), format!("c{target}")), region.clone())
                    })
                    .collect(),
            );
            let actual = must_evaluate_with(&states, &edges, &resets);
            check_execution_intersections(&actual, &edges, killed);
        }
    }
}

// Enumerate the finite product of program location and complete execution
// history bits. Saturation includes arbitrarily many loop traversals.
fn execution_intersections(edges: &BTreeSet<(usize, usize)>, killed: u8) -> Vec<BTreeSet<u8>> {
    let mut reached = vec![BTreeSet::new(); 3];
    let mut pending = vec![(0usize, 0u8)];
    while let Some((source, bits)) = pending.pop() {
        if !reached[source].insert(bits) {
            continue;
        }
        for (_, target) in edges.iter().filter(|(vertex, _)| *vertex == source) {
            let executed = bits | (1 << source);
            let next = if source >= *target {
                executed & !killed
            } else {
                executed
            };
            pending.push((*target, next));
        }
    }
    reached
}

fn check_execution_intersections(
    actual: &[BTreeSet<String>],
    edges: &BTreeSet<(usize, usize)>,
    killed: u8,
) {
    let expected = execution_intersections(edges, killed);
    for (vertex, histories) in expected.iter().enumerate() {
        if histories.is_empty() {
            continue;
        }
        let intersection = histories.iter().fold(7u8, |prior, bits| prior & bits);
        let names: BTreeSet<_> = (0..3)
            .filter(|bit| intersection & (1 << bit) != 0)
            .map(|index| format!("c{index}"))
            .collect();
        assert_eq!(
            actual[vertex], names,
            "edges={edges:?}, killed={killed}, vertex={vertex}"
        );
    }
}
