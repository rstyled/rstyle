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
