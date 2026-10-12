//! Exact per-source call summary inventory, retained on both documents.

use super::Graph;
use crate::{HirFnFacts, HirNode, NodeCategory, call_summaries};
use serde_json::{Value, json};

pub fn evidence(facts: &HirFnFacts, graph: &Graph) -> Result<Value, String> {
    let mut records = Vec::new();
    for operation in graph
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Call)
    {
        let node =
            find(&facts.body, operation.span).ok_or("Missing call source/HIR correspondence")?;
        let summary =
            call_summaries::resolve(node, facts).ok_or("Missing verified primitive summary")?;
        records.push(json!({"pdg_node_id":operation.id,"source_span":operation.span,"summary":summary,"operand_pdg_node_ids":operation.operands,"overflow_checks":facts.build.overflow_checks,"debug_assertions":facts.build.debug_assertions,"panic_strategy":facts.build.panic_strategy}));
    }
    Ok(json!(records))
}

fn find(node: &HirNode, span: crate::ByteSpan) -> Option<&HirNode> {
    if node.kind == "expr_method_call" && node.span == span {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, span))
}
