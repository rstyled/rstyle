//! Shared verification of concrete source-backed CALL records.
use crate::producer::PairView;

pub(super) struct Record<'a> {
    pub source: &'a crate::HirNode,
    pub node: &'a crate::PdgNode,
    pub summary: crate::call_summaries::Summary,
}

impl Record<'_> {
    pub fn verify_pure(
        self,
        pair: PairView<'_>,
        facts: &dyn crate::unit_store::EvidenceFacts,
        construct: &str,
    ) -> Result<(), String> {
        if self.node.attributes.construct.as_deref() != Some(construct)
            || self.node.original_kind.known_value() != Some(self.source.kind.as_str())
            || self
                .node
                .annotations
                .get("compat.rust.hir-r0:type")
                .and_then(|value| value.get("value"))
                .and_then(serde_json::Value::as_str)
                != Some(self.summary.result_type.as_str())
            || self.node.attributes.call_form != Some(crate::CallForm::Direct)
            || !self.node.attributes.reads.is_empty()
            || !self.node.attributes.writes.is_empty()
            || !self.node.attributes.defines.is_empty()
            || !self.node.attributes.symbol_ids.is_empty()
            || self.node.attributes.literal.is_some()
            || self.node.annotations.contains_key(crate::data_places::KEY)
        {
            return Err("Integer operation loses exact type, value semantics or effects".into());
        }
        self.verify(pair, facts)
    }

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
        let record = super::extension_record(pair, crate::call_summaries::KEY, &node.id)?;

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
        crate::helper_calls::verify_summary_operands(
            pair,
            crate::helper_calls::SummaryOperands {
                node,
                call: source,
                record,
            },
            facts,
        )
    }
}
