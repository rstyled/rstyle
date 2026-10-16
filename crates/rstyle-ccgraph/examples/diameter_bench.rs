//! Prebuild this harness; ordinary samples exclude fixture construction.
use rstyle_ccgraph::{CcLabel, ProjectedEdge, ProjectedGraph, ProjectedNode, wl};
use rstyle_pdg::EdgeFamily;
use std::{hint::black_box, time::Instant};

fn fixture(nodes: usize, fanout: bool) -> ProjectedGraph {
    let mut graph = ProjectedGraph {
        nodes: (0..nodes)
            .map(|id| ProjectedNode {
                id: id.to_string(),
                label: CcLabel::State,
            })
            .collect(),
        edges: Vec::new(),
    };
    for target in 1..nodes {
        let source = if fanout { 0 } else { target - 1 };
        for _ in 0..3 {
            graph.edges.push(ProjectedEdge {
                source: source.to_string(),
                target: target.to_string(),
                family: EdgeFamily::Data,
            });
        }
    }
    graph.edges.push(ProjectedEdge {
        source: "0".into(),
        target: "0".into(),
        family: EdgeFamily::Control,
    });
    graph
}

fn measure(nodes: usize, fanout: bool) {
    let graph = fixture(nodes, fanout);
    let expected = if fanout { 1 } else { nodes - 1 };
    assert_eq!(wl::diameter(&graph), expected);
    let mut samples = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        for _ in 0..100 {
            assert_eq!(black_box(wl::diameter(black_box(&graph))), expected);
        }
        samples.push(started.elapsed().as_secs_f64());
    }
    println!(
        "{}",
        serde_json::json!({
            "nodes": nodes, "fanout": fanout, "iterations": 100, "seconds": samples,
        })
    );
}

fn main() {
    for nodes in [32, 128, 512] {
        for fanout in [false, true] {
            measure(nodes, fanout);
        }
    }
}
