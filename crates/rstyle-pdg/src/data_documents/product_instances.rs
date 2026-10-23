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
        sources(facts, crate::product_instances::SpanArgument::Span)
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

fn sources(
    facts: &dyn EvidenceFacts,
    argument: crate::product_instances::SpanArgument,
) -> Result<Vec<&HirNode>, String> {
    if !activation::verify(facts)?
        || facts
            .product_instance()
            .is_none_or(|checked| checked.argument() != argument)
    {
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

pub(super) struct RangeOperations;
impl super::checked_instances::Operations for RangeOperations {
    const CONSTRUCT: &'static str = activation::RANGE_CONSTRUCT;
    const CAPABILITY: &'static str = crate::range_values::CONSUMING_CAPABILITY;
    const POLICY_ERROR: &'static str = "Range Into requires its consuming document policy";

    fn sources(facts: &dyn EvidenceFacts) -> Result<Vec<&HirNode>, String> {
        sources(facts, crate::product_instances::SpanArgument::RangeUsize)
    }

    fn verify_operation(
        pair: PairView<'_>,
        facts: &dyn EvidenceFacts,
        source: &HirNode,
    ) -> Result<(), String> {
        let dependency = crate::helper_calls::verify_use(source, facts)?;
        let summary = dependency.summary();
        if summary.argument_types.as_slice() != [crate::range_values::TOKEN]
            || summary.result_type != crate::data_aggregates::token(source)?
            || summary.mutates_arguments
            || summary.memory_effects
            || summary.may_panic
            || !summary.always_returns
        {
            return Err("Range Into changes its exact total consuming From contract".into());
        }

        let node = super::typed_operation(
            pair,
            source,
            activation::RANGE_CONSTRUCT,
            &summary.result_type,
        )?;
        if node.category != NodeCategory::Call {
            return Err("Range Into must retain its original whole-value CALL".into());
        }

        // The ordinary helper-document verifier checks the callee, operator,
        // dependency record and operand edges. A consuming CALL retains them.
        if node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != source
                .dispatch
                .as_ref()
                .map(|dispatch| dispatch.op.as_str())
            || node.attributes.literal.is_some()
            || !node.attributes.reads.is_empty()
            || !node.attributes.writes.is_empty()
            || !node.attributes.defines.is_empty()
            || !node.attributes.symbol_ids.is_empty()
            || node.annotations.contains_key(crate::data_places::KEY)
        {
            return Err("Range conversion invents memory or scalar effects".into());
        }

        Ok(())
    }
}
