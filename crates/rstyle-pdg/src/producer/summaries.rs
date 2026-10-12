//! Exact per-source call summary inventory, retained on both documents.

use super::Graph;
use crate::{HirFnFacts, HirNode, NodeCategory, call_summaries};
use serde_json::{Value, json};

pub fn evidence(facts: &HirFnFacts, graph: &Graph) -> Result<Value, String> {
    let capture = SummaryCapture { facts, graph };
    let records = capture.visit(&facts.body)?;
    let calls = graph
        .nodes
        .iter()
        .filter(|operation| operation.category == NodeCategory::Call)
        .count();
    if calls != records.len() {
        return Err("Source call and emitted CALL inventories disagree".into());
    }

    Ok(json!(records))
}

struct SummaryCapture<'a> {
    facts: &'a HirFnFacts,
    graph: &'a Graph,
}

impl SummaryCapture<'_> {
    fn visit(&self, node: &HirNode) -> Result<Vec<Value>, String> {
        let mut records = node
            .children
            .iter()
            .map(|child| self.visit(child))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if node.kind == "expr_method_call" {
            records.push(self.record(node)?);
        }

        Ok(records)
    }

    fn record(&self, node: &HirNode) -> Result<Value, String> {
        let operations: Vec<_> = self
            .graph
            .nodes
            .iter()
            .filter(|operation| {
                operation.category == NodeCategory::Call && operation.span == node.span
            })
            .collect();
        let [operation] = operations.as_slice() else {
            return Err("Ambiguous or missing call source/HIR correspondence".into());
        };
        let summary = call_summaries::resolve(node, self.facts)
            .ok_or("Missing verified primitive summary")?;

        serde_json::to_value(SummaryRecord {
            pdg_node_id: &operation.id,
            source_span: operation.span,
            summary,
            operand_pdg_node_ids: &operation.operands,
            overflow_checks: &self.facts.build.overflow_checks,
            debug_assertions: &self.facts.build.debug_assertions,
            panic_strategy: &self.facts.build.panic_strategy,
        })
        .map_err(|e| e.to_string())
    }
}

#[derive(serde::Serialize)]
struct SummaryRecord<'a> {
    pdg_node_id: &'a str,
    source_span: crate::ByteSpan,
    summary: &'a call_summaries::Summary,
    operand_pdg_node_ids: &'a [String],
    overflow_checks: &'a crate::BoolFact,
    debug_assertions: &'a crate::BoolFact,
    panic_strategy: &'a crate::StringFact,
}
