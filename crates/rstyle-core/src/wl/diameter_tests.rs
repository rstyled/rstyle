use super::{DIAMETER_LIMIT, MAX_LEVELS, diameter, graph};
use crate::facts::{NodeKind, Pdg};
use crate::pdgtest::{data, node};

type DiameterCase<'a> = (usize, &'a [(usize, usize)], usize);

fn input(nodes: usize, edges: &[(usize, usize)]) -> Pdg {
    Pdg {
        nodes: (0..nodes).map(|_| node(NodeKind::Param)).collect(),
        edges: edges.iter().map(|&(from, to)| data(from, to, 0)).collect(),
    }
}

/// Independent all-pairs shortest paths, treating recorded edges as undirected.
/// Unreachable pairs do not contribute to the maximum finite distance.
fn oracle(nodes: usize, edges: &[(usize, usize)]) -> usize {
    let unreachable = nodes + 1;
    let mut distances = vec![vec![unreachable; nodes]; nodes];
    for (at, row) in distances.iter_mut().enumerate() {
        row[at] = 0;
    }
    for &(from, to) in edges {
        if from < nodes && to < nodes && from != to {
            distances[from][to] = 1;
            distances[to][from] = 1;
        }
    }
    for via in 0..nodes {
        for from in 0..nodes {
            for to in 0..nodes {
                distances[from][to] =
                    distances[from][to].min(distances[from][via] + distances[via][to]);
            }
        }
    }
    distances
        .into_iter()
        .flatten()
        .filter(|distance| *distance != unreachable)
        .max()
        .unwrap_or(0)
}

#[test]
fn exhaustive_small_directed_graphs_match_undirected_shortest_paths() {
    for nodes in 0..=3 {
        for mask in 0usize..(1 << (nodes * nodes)) {
            let edges: Vec<_> = (0..nodes)
                .flat_map(|from| (0..nodes).map(move |to| (from, to)))
                .filter(|&(from, to)| mask & (1 << (from * nodes + to)) != 0)
                .collect();
            assert_eq!(
                diameter(&graph(&input(nodes, &edges))),
                oracle(nodes, &edges),
                "nodes={nodes}, mask={mask}"
            );
        }
    }
}

#[test]
fn undirected_paths_cycles_and_disconnected_components_keep_finite_distances() {
    let cases: &[DiameterCase<'_>] = &[
        (0, &[], 0),
        (1, &[(0, 0)], 0),
        (3, &[(0, 1), (2, 1)], 2),
        (5, &[(0, 1), (0, 2), (0, 3), (0, 4)], 2),
        (4, &[(0, 1), (0, 2), (1, 3), (2, 3)], 2),
        (5, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)], 2),
        (8, &[(0, 1), (1, 2), (2, 3), (4, 5), (5, 4), (6, 6)], 3),
    ];
    for &(nodes, edges, expected) in cases {
        assert_eq!(oracle(nodes, edges), expected);
        assert_eq!(diameter(&graph(&input(nodes, edges))), expected);
    }
}

#[test]
fn parallel_reverse_and_self_edges_do_not_lengthen_paths_or_lose_occurrences() {
    let edges: Vec<_> = (0..7)
        .flat_map(|at| std::iter::repeat_n((at, at + 1), 8).chain([(at + 1, at), (at, at)]))
        .collect();
    let made = graph(&input(8, &edges));
    assert_eq!(diameter(&made), 7);
    assert_eq!(oracle(8, &edges), 7);
    assert_eq!(made.inc.iter().map(Vec::len).sum::<usize>(), edges.len());
    assert_eq!(made.out.iter().map(Vec::len).sum::<usize>(), edges.len());
}

#[test]
fn graph_construction_still_filters_invalid_endpoints() {
    let valid = [(0, 1), (2, 1), (2, 2)];
    let mut edges = valid.to_vec();
    edges.extend([(3, 0), (0, 3), (usize::MAX, 1), (1, usize::MAX)]);
    let made = graph(&input(3, &edges));
    assert_eq!(made, graph(&input(3, &valid)));
    assert_eq!(diameter(&made), 2);
    assert_eq!(oracle(3, &edges), 2);
    assert_eq!(graph(&input(0, &[(0, 0)])), graph(&input(0, &[])));
}

#[test]
fn exact_diameter_limit_and_large_graph_fallback_remain_distinct() {
    assert_eq!(DIAMETER_LIMIT, 400);
    for nodes in [400, 401] {
        let edges: Vec<_> = (1..nodes).map(|to| (to - 1, to)).collect();
        let expected_chain = if nodes == 400 { 399 } else { MAX_LEVELS };
        let expected_isolated = if nodes == 400 { 0 } else { MAX_LEVELS };
        assert_eq!(diameter(&graph(&input(nodes, &edges))), expected_chain);
        assert_eq!(diameter(&graph(&input(nodes, &[]))), expected_isolated);
    }
}
