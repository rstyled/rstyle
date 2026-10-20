//! Shared verification of concrete source-backed CALL records.
use crate::producer::PairView;

pub(super) struct Record<'a> {
    pub source: &'a crate::HirNode,
    pub node: &'a crate::PdgNode,
    pub summary: crate::call_summaries::Summary,
}

impl Record<'_> {
    pub fn verify(
        self,
        pair: PairView<'_>,
        facts: &dyn crate::unit_store::EvidenceFacts,
    ) -> Result<(), String> {
        let Self {
            source,
            node,
            summary,
        } = self;
        let records = pair
            .pdg
            .extensions
            .get(crate::call_summaries::KEY)
            .and_then(serde_json::Value::as_array)
            .ok_or("Missing call summary inventory")?;
        let matched: Vec<_> = records
            .iter()
            .filter(|record| record["pdg_node_id"] == node.id)
            .collect();
        let [record] = matched.as_slice() else {
            return Err("Missing or duplicate registered summary".into());
        };

        crate::call_summaries::verify_build(record, facts)?;
        if record["summary"] != serde_json::to_value(&summary).map_err(|error| error.to_string())?
            || record.get("dependency").is_some()
            || record.get("library").is_some()
            || node
                .attributes
                .callee
                .as_ref()
                .and_then(crate::StringFact::known_value)
                != Some(&summary.callee_path)
            || node
                .attributes
                .operator
                .as_ref()
                .and_then(crate::StringFact::known_value)
                != Some(&summary.callee_path)
            || record["source_span"]
                != serde_json::to_value(source.span).map_err(|error| error.to_string())?
        {
            return Err("Registered summary disagrees with concrete authority".into());
        }
        crate::helper_calls::verify_call_operands(pair, node, source, record)
    }
}
