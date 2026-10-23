//! Complete checked AsRef inventory, verified independently of lowering.
use crate::reference_instances::activation;
use crate::{HirNode, NodeCategory, producer::PairView, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = sources(facts)?;
    super::verify_construct_inventory(pair, [activation::CONSTRUCT], [sources.len()])?;
    if !sources.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, activation::CAPABILITY)
    {
        return Err("Checked AsRef requires its registered serialized document policy".into());
    }

    for source in sources {
        verify_operation(pair, facts, source)?;
    }

    Ok(())
}

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
    let node = super::typed_operation(pair, source, activation::CONSTRUCT, &summary.result_type)?;
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
