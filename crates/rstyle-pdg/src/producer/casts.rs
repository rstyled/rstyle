//! Resolved integer cast facts, separate from the unchanged matching labels.

use super::Graph;
use serde_json::{Value, json};

pub const KEY: &str = "rstyle.hir-coverage:integer-casts";

pub fn evidence(
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &Graph,
) -> Result<Value, String> {
    let width = facts
        .build
        .pointer_width
        .value
        .ok_or("Missing cast target width")?;
    let casts = graph.nodes.iter().filter(|node| {
        node.original_kind == "expr_cast" && node.category == crate::NodeCategory::Computation
    });
    let records: Result<Vec<_>, String> = casts
        .map(|node| {
            let [operand] = node.operands.as_slice() else {
                return Err("Integer cast must retain exactly one value dependency".into());
            };
            let input = graph
                .nodes
                .iter()
                .find(|input| &input.id == operand)
                .ok_or("Missing cast operand value")?;
            Ok(json!({
                "pdg_node_id": node.id,
                "operand_pdg_node_id": operand,
                "operand_type": input.ty,
                "result_type": node.ty,
                "target_pointer_width": width,
                "semantics": "rust-integer-as/1",
                "span": node.span
            }))
        })
        .collect();

    Ok(json!(records?))
}
