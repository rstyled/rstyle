//! Independently bind the identity CALL to its complete typed source operand.
use crate::product_instances::activation;
use crate::{HirNode, NodeCategory, producer::PairView, unit_store::EvidenceFacts};

pub(super) struct Operations;

impl super::checked_instances::Operations for Operations {
    const CONSTRUCT: &'static str = activation::CONSTRUCT;
    const CAPABILITY: &'static str = activation::CAPABILITY;
    const POLICY_ERROR: &'static str =
        "Product identity requires its registered serialized document policy";

    fn sources(facts: &dyn EvidenceFacts) -> Result<Vec<&HirNode>, String> {
        if !activation::verify(facts)? {
            return Ok(Vec::new());
        }

        let mut sources = Vec::new();
        for source in
            crate::field_locations::operations(&facts.body, &["expr_call", "expr_method_call"])
        {
            if activation::operation(source, facts)? {
                sources.push(source);
            }
        }
        if sources.len() != 1 {
            return Err("Product constructor loses its exact one-conversion inventory".into());
        }
        Ok(sources)
    }

    fn verify_operation(
        pair: PairView<'_>,
        facts: &dyn EvidenceFacts,
        source: &HirNode,
    ) -> Result<(), String> {
        let summary = activation::summary(source, facts)?
            .ok_or("Product conversion lost its independently checked identity summary")?;
        if summary.argument_types.as_slice() != [summary.result_type.as_str()] {
            return Err("Product identity changes its entire nominal input/output value".into());
        }
        let node =
            super::typed_operation(pair, source, activation::CONSTRUCT, &summary.result_type)?;
        if node.category != NodeCategory::Call {
            return Err(
                "Product conversion must retain its exact whole-value CALL operation".into(),
            );
        }

        super::summary_records::Record {
            source,
            node,
            summary,
        }
        .verify_pure(pair, facts, activation::CONSTRUCT)
    }
}
