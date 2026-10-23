//! Complete checked AsRef inventory, verified independently of lowering.
use crate::reference_instances::activation;
use crate::{HirNode, NodeCategory, producer::PairView, unit_store::EvidenceFacts};

pub(super) struct Operations;

impl super::checked_instances::Operations for Operations {
    const CONSTRUCT: &'static str = activation::CONSTRUCT;
    const CAPABILITY: &'static str = activation::CAPABILITY;
    const POLICY_ERROR: &'static str =
        "Checked AsRef requires its registered serialized document policy";

    fn sources(facts: &dyn EvidenceFacts) -> Result<Vec<&HirNode>, String> {
        if !activation::verify(facts)? {
            return Ok(Vec::new());
        }

        let mut sources = Vec::new();
        for source in crate::field_locations::operations(&facts.body, &["expr_method_call"]) {
            if activation::operation(source, facts)?.is_some() {
                sources.push(source);
            }
        }
        if sources.len() != 1 {
            return Err("Checked constructor loses its exact one-operation AsRef inventory".into());
        }

        Ok(sources)
    }

    fn verify_operation(
        pair: PairView<'_>,
        facts: &dyn EvidenceFacts,
        source: &HirNode,
    ) -> Result<(), String> {
        let summary = crate::call_summaries::reference_constructor(source, facts)?
            .ok_or("Checked AsRef lost its independent call summary")?;
        if summary.id == activation::STRING_SUMMARY
            && !crate::extraction_profile::policy_supports(
                &pair.pdg.profile,
                activation::STRING_CAPABILITY,
            )
        {
            return Err(
                "String byte-view CALL requires its registered serialized document policy".into(),
            );
        }

        let node =
            super::typed_operation(pair, source, activation::CONSTRUCT, &summary.result_type)?;
        if node.category != NodeCategory::Call {
            return Err("Checked AsRef must retain its exact CALL operation".into());
        }
        super::summary_records::Record {
            source,
            node,
            summary,
        }
        .verify_pure(pair, facts, activation::CONSTRUCT)
    }
}
