//! Builders that materialize compact stage fixtures as typed inputs.

use rstyle_ccgraph::*;
use rstyle_pdg::{EdgeFamily, NodeCategory};

pub fn vendor_path(name: &str) -> String {
    format!("{}/../../vendor-compat/{name}", env!("CARGO_MANIFEST_DIR"))
}

pub fn load_vendor(name: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(vendor_path(name)).expect("vendored snapshot is committed");
    serde_json::from_str(&text).expect("vendored snapshot is valid JSON")
}

/// Build a stage graph from compact fixture encodings.
pub fn compact_graph(nodes: &serde_json::Value, edges: &serde_json::Value) -> ImportedFunction {
    let unit = InterfaceElement {
        role: "return".to_string(),
        normalized_type: "unit".to_string(),
        variadic: false,
    };
    ImportedFunction {
        key: "stage".to_string(),
        nodes: compact_nodes(nodes),
        edges: compact_edges(edges),
        inputs: Vec::new(),
        outputs: vec![unit],
        name: Some("stage".to_string()),
        reference_count: Some(0),
        reference_policy_ok: true,
        target_pointer_width: Some(64),
        native_width_involved: false,
    }
}

fn compact_nodes(nodes: &serde_json::Value) -> Vec<ImportedNode> {
    nodes
        .as_object()
        .expect("nodes object")
        .iter()
        .map(|(alias, label)| ImportedNode {
            id: alias.clone(),
            category: fixture_category(label.as_str().expect("label")),
            administrative: false,
            reachable: true,
            defines_binding: false,
            reads_or_writes: false,
            value_site_reachable: true,
            read_symbols: Vec::new(),
            registered_meaning: None,
        })
        .collect()
}

fn compact_edges(edges: &serde_json::Value) -> Vec<ImportedEdge> {
    edges
        .as_array()
        .expect("edges array")
        .iter()
        .enumerate()
        .map(|(index, edge)| ImportedEdge {
            source: edge[0].as_str().expect("source").to_string(),
            target: edge[1].as_str().expect("target").to_string(),
            family: parse_family(edge[2].as_str().expect("family")),
            origin: format!("stage-edge-{index}"),
        })
        .collect()
}

fn fixture_category(token: &str) -> NodeCategory {
    match token {
        "DECL" => NodeCategory::Declaration,
        "ASSIGN" => NodeCategory::Assignment,
        "CTRL" => NodeCategory::Control,
        "STATE" => NodeCategory::Call,
        "OTHER" => NodeCategory::Computation,
        _ => panic!("unknown stage label {token:?}"),
    }
}

pub fn parse_family(token: &str) -> EdgeFamily {
    match token {
        "data" => EdgeFamily::Data,
        "control" => EdgeFamily::Control,
        "execution" => EdgeFamily::Execution,
        _ => panic!("unknown edge family {token:?}"),
    }
}

pub fn parse_category(token: &str) -> NodeCategory {
    match token {
        "entry" => NodeCategory::Entry,
        "exit" => NodeCategory::Exit,
        "formal_input" => NodeCategory::FormalInput,
        "declaration" => NodeCategory::Declaration,
        "read" => NodeCategory::Read,
        "assignment" => NodeCategory::Assignment,
        "computation" => NodeCategory::Computation,
        "control" => NodeCategory::Control,
        "call" => NodeCategory::Call,
        "return" => NodeCategory::Return,
        "merge" => NodeCategory::Merge,
        "other" => NodeCategory::Other,
        _ => panic!("unknown category {token:?}"),
    }
}
