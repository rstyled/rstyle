use super::*;

#[test]
fn sibling_backedges_reset_the_union_of_continue_paths() {
    let resets = derive(&[(0, 1), (1, 2), (1, 3), (2, 1), (3, 1), (1, 4)]).unwrap();
    assert_eq!(resets.region("2", "1"), Some(&names(&[1, 2, 3])));
    assert_eq!(resets.region("3", "1"), Some(&names(&[1, 2, 3])));
    assert_eq!(resets.region("1", "4"), None);
}

#[test]
fn nested_backedges_preserve_outer_values_until_outer_iteration() {
    let resets = derive(&[(0, 1), (1, 2), (2, 3), (3, 2), (2, 4), (4, 1), (1, 5)]).unwrap();
    assert_eq!(resets.region("3", "2"), Some(&names(&[2, 3])));
    assert_eq!(resets.region("4", "1"), Some(&names(&[1, 2, 3, 4])));
}

#[test]
fn self_loops_and_acyclic_paths_have_exact_reset_boundaries() {
    let resets = derive(&[(0, 1), (1, 1), (1, 2)]).unwrap();
    assert_eq!(resets.region("1", "1"), Some(&names(&[1])));
    assert!(
        derive(&[(0, 1), (0, 2), (1, 3), (2, 3)])
            .unwrap()
            .0
            .is_empty()
    );
}

#[test]
fn two_entry_cycle_is_not_certified_as_a_natural_loop() {
    assert!(derive(&[(0, 1), (0, 2), (1, 2), (2, 1), (2, 3)]).is_err());
}

fn names(vertices: &[usize]) -> BTreeSet<String> {
    vertices.iter().map(usize::to_string).collect()
}

fn derive(input: &[(usize, usize)]) -> Result<Resets, String> {
    let vertices: BTreeSet<_> = input.iter().flat_map(|(a, b)| [*a, *b]).collect();
    // Independent dominance oracle: deleting a dominator makes its dominated
    // vertex unreachable. This does not reuse production dominance iteration.
    let dominance = vertices
        .iter()
        .map(|vertex| {
            let dominators = vertices
                .iter()
                .filter(|removed| !reachable_without(input, **removed).contains(vertex))
                .map(usize::to_string)
                .collect();
            (vertex.to_string(), dominators)
        })
        .collect();
    let edges: Vec<_> = input
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();

    Resets::derive(&dominance, &edges)
}

fn reachable_without(edges: &[(usize, usize)], removed: usize) -> BTreeSet<usize> {
    let mut visited = BTreeSet::new();
    let mut pending = vec![0];
    while let Some(vertex) = pending.pop() {
        if vertex == removed || !visited.insert(vertex) {
            continue;
        }
        pending.extend(
            edges
                .iter()
                .filter(|(source, _)| *source == vertex)
                .map(|(_, target)| *target),
        );
    }

    visited
}
