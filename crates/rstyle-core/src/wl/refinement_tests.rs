use super::{Graph, IN_SALT, Nb, OUT_SALT, combine, refine};

fn neighbour(node: usize, tag: u64) -> Nb {
    Nb { node, tag }
}

fn heterogeneous_graph() -> Graph {
    Graph {
        labels: vec![11, 23, 37, 41, 59],
        inc: vec![
            vec![neighbour(2, 29)],
            vec![neighbour(0, 17), neighbour(1, 43), neighbour(0, 17)],
            vec![],
            vec![neighbour(2, 61)],
            vec![],
        ],
        out: vec![
            vec![neighbour(1, 17), neighbour(1, 17)],
            vec![neighbour(1, 43)],
            vec![neighbour(3, 61), neighbour(0, 29)],
            vec![],
            vec![],
        ],
    }
}

fn ordered_pair(seed: u64, a: u64, b: u64) -> u64 {
    combine(combine(seed, a.min(b)), a.max(b))
}

fn repeated_and_single(seed: u64, repeated: u64, single: u64) -> u64 {
    if repeated <= single {
        combine(combine(combine(seed, repeated), repeated), single)
    } else {
        combine(combine(combine(seed, single), repeated), repeated)
    }
}

/// These expressions spell out the five nodes' actual incoming/outgoing multisets.
/// They neither inspect adjacency nor share the production scratch-buffer loop.
fn expected_round(previous: &[u64]) -> Vec<u64> {
    let incoming0 = combine(combine(previous[0], IN_SALT), combine(29, previous[2]));
    let outgoing0 = combine(combine(incoming0, OUT_SALT), combine(17, previous[1]));
    let incoming1 = repeated_and_single(
        combine(previous[1], IN_SALT),
        combine(17, previous[0]),
        combine(43, previous[1]),
    );
    let incoming2 = combine(previous[2], IN_SALT);
    let incoming3 = combine(combine(previous[3], IN_SALT), combine(61, previous[2]));
    vec![
        combine(outgoing0, combine(17, previous[1])),
        combine(combine(incoming1, OUT_SALT), combine(43, previous[1])),
        ordered_pair(
            combine(incoming2, OUT_SALT),
            combine(29, previous[0]),
            combine(61, previous[3]),
        ),
        combine(incoming3, OUT_SALT),
        combine(combine(previous[4], IN_SALT), OUT_SALT),
    ]
}

#[test]
fn refinement_preserves_explicit_multisets_across_empty_lists_and_rounds() {
    let graph = heterogeneous_graph();
    let original = graph.clone();
    let mut previous = graph.labels.clone();
    for round in 1..=4 {
        let expected = expected_round(&previous);
        let actual = refine(&graph, &previous);
        assert_eq!(actual, expected, "round {round}");
        previous = actual;
    }
    assert_eq!(graph, original);
}

#[test]
fn refinement_preserves_edge_order_independence_and_parallel_multiplicity() {
    let original = heterogeneous_graph();
    let mut reordered = original.clone();
    for neighbours in reordered.inc.iter_mut().chain(&mut reordered.out) {
        neighbours.reverse();
    }
    let expected = expected_round(&original.labels);
    assert_eq!(refine(&reordered, &original.labels), expected);

    let mut fewer = original.clone();
    fewer.out[0].pop();
    fewer.inc[1].pop();
    let changed = refine(&fewer, &original.labels);
    assert_ne!(changed[0], expected[0]);
    assert_ne!(changed[1], expected[1]);
    assert_eq!(&changed[2..], &expected[2..]);
}

#[test]
fn refinement_follows_node_permutation_without_changing_labels() {
    let original = heterogeneous_graph();
    let permutation = [3, 0, 4, 1, 2];
    let mut moved = original.clone();
    for (old, &new) in permutation.iter().enumerate() {
        moved.labels[new] = original.labels[old];
        for (source, target) in [
            (&original.inc, &mut moved.inc),
            (&original.out, &mut moved.out),
        ] {
            target[new] = source[old]
                .iter()
                .map(|edge| neighbour(permutation[edge.node], edge.tag))
                .collect();
        }
    }
    let mut previous = original.labels.clone();
    let mut moved_previous = moved.labels.clone();
    for _ in 0..4 {
        previous = refine(&original, &previous);
        moved_previous = refine(&moved, &moved_previous);
        for (old, &new) in permutation.iter().enumerate() {
            assert_eq!(moved_previous[new], previous[old]);
        }
    }
}
