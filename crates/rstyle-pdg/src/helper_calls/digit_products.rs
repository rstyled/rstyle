//! Complete checked parser graphs retain conditional reads and tagged components.
//! This receipt does not grant a caller ownership or result-consumption authority.
use super::*;
use crate::{
    ExtractionStatus, data_origins::Origin, data_types::TypeToken, field_locations::Projection,
    helper_component_outputs::Output, helper_reads::Read, scalar_product_instances::activation,
    slice_patterns::graph,
};

pub(crate) const CAPABILITY: &str = "rust.checked-digit-product-summaries";
const SUMMARY: &str = "rstyle.helper.guarded-digit-product/1";
const DEPENDENCE: &str = "guarded-prefix-product-with-verified-callee-return-graph/1";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && activation::candidate(facts)
        && facts.body.kind == "expr_block"
        && matches!(facts.body.children.as_slice(), [matched] if matched.kind == "expr_match")
}

pub(super) fn certify(
    facts: &dyn EvidenceFacts,
) -> Result<Option<(Summary, producer::Pair)>, String> {
    // A raw match shape cannot substitute for the complete source receipt.
    if activation::verify(facts).is_err() {
        return Ok(None);
    }
    let pair = producer::produce(facts).map_err(|errors| errors.join("; "))?;
    let summary = crate::diagnostics::Stage::new("helper.digit-product-summary", &facts.name, None)
        .observe(|| derive(facts, &pair))?;

    Ok(Some((summary, pair)))
}

pub(super) fn verify(
    facts: &dyn EvidenceFacts,
    pair: &producer::Pair,
    actual: &Summary,
) -> Result<(), String> {
    if actual != &derive(facts, pair)? {
        return Err("Digit helper summary changes its guarded result or physical reads".into());
    }
    Ok(())
}

fn derive(facts: &dyn EvidenceFacts, pair: &producer::Pair) -> Result<Summary, String> {
    if !candidate(facts)
        || !crate::extraction_profile::policy_supports(&pair.pdg.profile, CAPABILITY)
        || pair.cfg.extraction_evidence.status != ExtractionStatus::Complete
    {
        return Err("Digit helper requires its complete registered parser contract".into());
    }
    let checked = activation::verify(facts)?;
    crate::scalar_product_instances::origins::documents::verify(pair.as_view(), &checked, facts)?;
    let replay = graph::verify(pair.as_view(), checked.prefix())?;
    let reads = reads(pair, &checked, &replay)?;
    let suffix = Output {
        selectors: vec![0, 0],
        ty: TypeToken::from_tree(&checked.instance().fields[0].ty),
        origins: vec![Origin {
            parameter: 0,
            path: vec![
                Projection::SharedReferent,
                Projection::PrefixSuffix {
                    start: checked.prefix().consumed(),
                },
            ],
        }],
    };

    Ok(Summary {
        identity: SUMMARY.into(),
        argument_types: argument_types(facts)?,
        result_type: result_type(facts)?,
        mutates_arguments: false,
        memory_effects: true,
        may_panic: false,
        always_returns: true,
        argument_evaluation: "source-order-once/1".into(),
        result_dependence: DEPENDENCE.into(),
        bounded_loop: None,
        reference_outputs: None,
        component_outputs: Some(vec![suffix]),
        memory_reads: Some(reads),
        tagged_output: None,
    })
}

fn reads(
    pair: &producer::Pair,
    checked: &activation::Checked<'_>,
    replay: &graph::Replay<'_>,
) -> Result<Vec<Read>, String> {
    let mut reads = Vec::new();
    for ((selector, region), digit) in replay.selector_reads.iter().zip(checked.prefix().digits()) {
        let origin = region.checked_origin(checked.prefix())?;
        let body = crate::data_documents::value_operation(pair.as_view(), digit.read())?;
        for operation in [*selector, body] {
            reads.push(read(operation, &origin, pair)?);
        }
    }
    let observed = pair
        .pdg
        .nodes
        .iter()
        .flat_map(|node| &node.attributes.reads)
        .filter(|access| access.location_id.is_some())
        .count();
    if observed != reads.len() || reads.len() != usize::from(checked.prefix().consumed()) * 2 {
        return Err(
            "Digit helper omits or duplicates conditional physical read occurrences".into(),
        );
    }

    Ok(reads)
}

fn read(
    operation: &crate::PdgNode,
    origin: &Origin,
    pair: &producer::Pair,
) -> Result<Read, String> {
    let [access] = operation.attributes.reads.as_slice() else {
        return Err("Digit helper requires one physical access per source read".into());
    };
    let location = pair
        .pdg
        .memory_locations
        .iter()
        .find(|location| Some(&location.id) == access.location_id.as_ref())
        .ok_or("Digit helper read lacks its exact physical location")?;
    let evidence = &location.extensions[crate::data_documents::LOCATION_KEY];
    let path = serde_json::to_value(&origin.path).map_err(|error| error.to_string())?;
    let value = TypeToken::Scalar("u8".into());
    if location.abstraction != "rstyle.formal-prefix-element/1"
        || evidence["parameter"] != origin.parameter
        || evidence["path"] != path
        || evidence["type"] != serde_json::to_value(&value).map_err(|error| error.to_string())?
    {
        return Err("Digit helper physical read changes its saved prefix element".into());
    }

    Ok(Read {
        operation: operation.id.clone(),
        access: 0,
        parameter: origin.parameter,
        path: origin.path.clone(),
        value,
    })
}
