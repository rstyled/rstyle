//! Selected reference leaves of complete aggregate helper results.
//! This transports verified origins; it grants no Copy, cleanup or foreign authority.
mod roster;
#[cfg(test)]
mod tests;

use crate::{
    HirNode, NodeCategory, data_origins::Origin, data_types::TypeToken,
    field_locations::Projection, producer::PairView, unit_store::EvidenceFacts,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CAPABILITY: &str = "rust.component-helper-results";
pub const SUMMARY: &str = "rstyle.helper.component-return/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub selectors: Vec<usize>,
    pub ty: TypeToken,
    pub origins: Vec<Origin>,
}

pub fn used(facts: &dyn EvidenceFacts) -> bool {
    crate::helper_calls::call_nodes(facts)
        .into_iter()
        .any(|call| crate::helper_calls::direct_resolution(call).is_some() && aggregate_call(call))
}

fn aggregate_call(call: &HirNode) -> bool {
    crate::data_aggregates::token(call)
        .ok()
        .and_then(|text| crate::data_types::parse(text).ok())
        .is_some_and(|ty| {
            matches!(ty, TypeToken::Tuple(_) | TypeToken::Struct { .. })
                && crate::data_origins::shared(&ty)
        })
}

pub fn call_used(call: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY) && aggregate_call(call)
}

pub fn supported(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && shared_formals(facts)
        && crate::helper_references::result_type(facts)
            .is_ok_and(|ty| roster::leaves(&ty).is_ok_and(|leaves| !leaves.is_empty()))
}

fn shared_formals(facts: &dyn EvidenceFacts) -> bool {
    facts.signature.params.iter().all(|parameter| {
        parameter
            .ty
            .normalized
            .as_deref()
            .and_then(|token| crate::data_types::parse(token).ok())
            .is_some_and(|ty| {
                !crate::data_origins::shared(&ty) || matches!(ty, TypeToken::Shared(_))
            })
    })
}

pub fn derive(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
) -> Result<Option<Vec<Output>>, String> {
    if !supported(facts) {
        return Ok(None);
    }
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
        || !pair.pdg.memory_locations.is_empty()
    {
        return Err("Component helper results require complete memory-free return evidence".into());
    }

    let mut outputs = roster::leaves(&crate::helper_references::result_type(facts)?)?;
    let returns: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Return)
        .collect();
    if returns.is_empty() {
        return Err("Component result lacks returning evidence".into());
    }
    for output in &mut outputs {
        output.origins = output_origins(output, &returns, pair, facts)?;
    }
    Ok(Some(outputs))
}

fn output_origins(
    output: &Output,
    returns: &[&crate::PdgNode],
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    let mut origins = BTreeSet::new();
    for returned in returns {
        for origin in crate::data_documents::origins::selected_return(
            returned,
            &output.selectors,
            pair,
            facts,
        )? {
            verify_origin(&origin, facts, &output.ty)?;
            origins.insert(origin);
        }
    }
    Ok(origins.into_iter().collect())
}

fn verify_origin(origin: &Origin, facts: &dyn EvidenceFacts, ty: &TypeToken) -> Result<(), String> {
    if origin.path.first() != Some(&Projection::SharedReferent) {
        return Err("Component return requires an existing shared formal referent".into());
    }
    crate::helper_references::verify_formal(origin, facts, ty)
}

pub fn map(
    call: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
    mut resolve: impl FnMut(u32) -> Result<Vec<Origin>, String>,
) -> Result<Vec<Origin>, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Selected component call requires its registered capability".into());
    }
    let dependency = crate::helper_calls::verify_use(call, facts)?;
    let outputs = dependency
        .summary()
        .component_outputs
        .as_ref()
        .ok_or("Missing verified component output summary")?;
    let ty = crate::data_types::parse(crate::data_aggregates::token(call)?)?;
    roster::verify(&ty, outputs)?;

    let mut roots = BTreeSet::new();
    for output in roster::selected(outputs, selectors)? {
        for origin in &output.origins {
            roots.extend(crate::helper_references::project_output(
                origin,
                resolve(origin.parameter)?,
            )?);
        }
    }
    Ok(roots.into_iter().collect())
}
