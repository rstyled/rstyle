//! Exact per-source call summary inventory, retained on both documents.

use super::Graph;
use crate::{HirNode, NodeCategory, call_summaries};
use serde_json::{Value, json};

pub fn evidence(
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &Graph,
) -> Result<Value, String> {
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
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    graph: &'a Graph,
}

impl SummaryCapture<'_> {
    fn visit(&self, node: &HirNode) -> Result<Vec<Value>, String> {
        if crate::result_maps::candidate(node, self.facts) {
            let checked = crate::result_maps::verify(node, self.facts)?;
            return self.visit(checked.receiver);
        }
        if crate::iterator_position::candidate(node, self.facts) {
            let checked = crate::iterator_position::verify(node, self.facts)?;
            return self.visit(checked.receiver);
        }
        if crate::option_propagation::candidate(node, self.facts) {
            let checked = crate::option_propagation::verify(node, self.facts)?;
            return self.visit(checked.operand);
        }
        if crate::product_result_propagation::candidate(node, self.facts) {
            let checked = crate::product_result_propagation::verify(node, self.facts)?;
            return self.visit(checked.operand);
        }
        if crate::scalar_result_propagation::candidate(node, self.facts) {
            let checked = crate::scalar_result_propagation::verify(node, self.facts)?;
            return self.visit(checked.operand);
        }
        if crate::borrowed_result_propagation::candidate(node, self.facts) {
            let checked = crate::borrowed_result_propagation::verify(node, self.facts)?;
            return self.visit(checked.operand);
        }
        if crate::result_propagation::candidate(node, self.facts) {
            let checked = crate::result_propagation::verify(node, self.facts)?;
            return self.visit(checked.operand);
        }
        let mut records = node
            .children
            .iter()
            .map(|child| self.visit(child))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if crate::named_callbacks::function_item(node) {
            records.push(self.record(node)?);
        }
        if matches!(node.kind.as_str(), "expr_call" | "expr_method_call")
            && !crate::checked_options::constructor_candidate(node, self.facts)
            && !crate::concrete_results::constructor_candidate(node, self.facts)
            && !crate::data_enums::constructor_candidate(node, self.facts)
            && !crate::option_callbacks::candidate(node, self.facts)
        {
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
        if (node.kind == "expr_call" && !call_summaries::static_intrinsic(node, self.facts))
            || crate::helper_calls::selected_resolution(node, self.facts).is_some()
        {
            return serde_json::to_value(HelperRecord {
                pdg_node_id: &operation.id,
                source_span: node.span,
                operand_pdg_node_ids: &operation.operands,
                dependency: crate::helper_calls::verify_use(node, self.facts)?.wire()?,
            })
            .map_err(|error| error.to_string());
        }
        let summary = call_summaries::resolve_for(node, self.facts)?
            .ok_or("Missing verified method summary")?;
        let library = if crate::data_library::operation(node).is_some() {
            Some(crate::data_library::evidence(node, self.facts)?)
        } else {
            None
        };

        serde_json::to_value(SummaryRecord {
            pdg_node_id: &operation.id,
            source_span: operation.span,
            summary: &summary,
            library,
            operand_pdg_node_ids: &operation.operands,
            overflow_checks: &self.facts.build.overflow_checks,
            debug_assertions: &self.facts.build.debug_assertions,
            panic_strategy: &self.facts.build.panic_strategy,
        })
        .map_err(|e| e.to_string())
    }
}

#[derive(serde::Serialize)]
struct HelperRecord<'a> {
    pdg_node_id: &'a str,
    source_span: crate::ByteSpan,
    operand_pdg_node_ids: &'a [String],
    dependency: Value,
}

#[derive(serde::Serialize)]
struct SummaryRecord<'a> {
    pdg_node_id: &'a str,
    source_span: crate::ByteSpan,
    summary: &'a call_summaries::Summary,
    #[serde(skip_serializing_if = "Option::is_none")]
    library: Option<Value>,
    operand_pdg_node_ids: &'a [String],
    overflow_checks: &'a crate::BoolFact,
    debug_assertions: &'a crate::BoolFact,
    panic_strategy: &'a crate::StringFact,
}
