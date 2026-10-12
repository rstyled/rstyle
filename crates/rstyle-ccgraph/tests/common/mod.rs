//! Shared builders for pipeline gate tests.

use rstyle_ccgraph::*;
use rstyle_pdg::{EdgeFamily, NodeCategory};
use std::collections::BTreeMap;

pub fn element(role: &str, kind: &str) -> InterfaceElement {
    InterfaceElement {
        role: role.to_string(),
        normalized_type: kind.to_string(),
        variadic: false,
    }
}

pub fn plain_node(id: &str, category: NodeCategory) -> ImportedNode {
    ImportedNode {
        id: id.to_string(),
        category,
        administrative: false,
        reachable: true,
        defines_binding: false,
        reads_or_writes: false,
        value_site_reachable: true,
        read_symbols: Vec::new(),
        registered_meaning: None,
    }
}

pub fn stage_function(
    key: &str,
    nodes: Vec<ImportedNode>,
    edges: Vec<(&str, &str, EdgeFamily)>,
    name: &str,
) -> ImportedFunction {
    let reference_count = distinct_reads(&nodes);
    ImportedFunction {
        extraction_context: None,
        key: key.to_string(),
        nodes,
        edges: edges
            .into_iter()
            .enumerate()
            .map(|(index, (source, target, family))| ImportedEdge {
                source: source.to_string(),
                target: target.to_string(),
                family,
                origin: format!("stage-edge-{index}"),
            })
            .collect(),
        inputs: vec![element("parameter", "i32")],
        outputs: vec![element("return", "i32")],
        name: Some(name.to_string()),
        reference_count: Some(reference_count),
        target_pointer_width: Some(64),
        native_width_involved: false,
    }
}

fn distinct_reads(nodes: &[ImportedNode]) -> u64 {
    let mut symbols = BTreeMap::new();
    for node in nodes {
        if node.category == NodeCategory::Read {
            for symbol in &node.read_symbols {
                symbols.insert(symbol.clone(), true);
            }
        }
    }
    symbols.len() as u64
}
