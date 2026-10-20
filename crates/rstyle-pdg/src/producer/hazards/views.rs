//! Normal-flow limitations for checked array views and their verified callers.

use super::*;

pub(super) struct Views {
    enabled: bool,
    calls: Vec<crate::ByteSpan>,
}

impl Views {
    pub(super) fn new(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Self, String> {
        let enabled = crate::extraction_profile::facts_support(
            facts,
            crate::data_subslices::ARRAY_CAPABILITY,
        );
        let mut calls = Vec::new();
        if enabled {
            for call in crate::helper_calls::call_nodes(facts) {
                if crate::helper_calls::verify_use(call, facts)?
                    .summary()
                    .may_panic
                {
                    calls.push(call.span);
                }
            }
        }
        Ok(Self { enabled, calls })
    }

    pub(super) fn relevant(&self, node: &Operation, graph: &Graph) -> bool {
        self.enabled
            && (array_length(node, graph).is_some()
                || node.category == NodeCategory::Call && self.calls.contains(&node.span))
    }

    pub(super) fn safe(&self, node: &Operation, graph: &Graph, operands: &[Option<i128>]) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(length) = array_length(node, graph) else {
            return false;
        };
        let bounds = match operands {
            [_, Some(start), Some(end)] => Some((*start, *end)),
            [_, Some(end)] => Some((0, *end)),
            _ => None,
        };
        bounds.is_some_and(|(start, end)| 0 <= start && start <= end && end <= i128::from(length))
    }
}

fn array_length(node: &Operation, graph: &Graph) -> Option<u64> {
    if node.construct.as_deref() != Some("data.subslice-place") {
        return None;
    }
    let base = graph
        .nodes
        .iter()
        .find(|base| node.operands.first() == Some(&base.id))?;
    let crate::data_types::TypeToken::Array { length, .. } =
        crate::data_types::parse(&base.ty).ok()?
    else {
        return None;
    };
    Some(length)
}
