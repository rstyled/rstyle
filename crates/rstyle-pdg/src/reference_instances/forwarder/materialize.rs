//! Concrete forwarding facts retain the complete checked constructor dependency.
//! The selected association authorizes only the exact preserved source call.
#[cfg(test)]
mod tests;

use crate::reference_instances::{CheckedProjection, Haystack, ReferenceObservation};
use crate::{HirFnFacts, HirNode};

type Result<T> = std::result::Result<T, String>;

/// Neither serialized observations nor normalized facts can mint this witness.
pub struct CheckedForwarderProjection {
    facts: HirFnFacts,
    constructor: CheckedProjection,
    call: HirNode,
}

impl CheckedForwarderProjection {
    pub fn facts(&self) -> &HirFnFacts {
        &self.facts
    }

    pub fn constructor(&self) -> &CheckedProjection {
        &self.constructor
    }

    pub(crate) fn call(&self) -> &HirNode {
        &self.call
    }
}

pub fn verify(
    forwarder: ReferenceObservation<'_>,
    constructor: ReferenceObservation<'_>,
    haystack: Haystack,
) -> Result<CheckedForwarderProjection> {
    let canonical = forwarder.canonical;
    let dependency = crate::reference_instances::verify_constructor_projection(
        constructor.canonical,
        constructor.observation,
        haystack,
    )?;
    super::verify(forwarder, constructor, haystack.spelling())?;
    crate::nominal_boundary::resolution::probe_reference_product_result(canonical)?;

    let result = dependency
        .facts()
        .signature
        .return_ty
        .normalized
        .as_deref()
        .ok_or("Checked From dependency lacks its complete result token")?;
    let facts = concrete_facts(canonical, haystack, result)?;
    let call = facts.body.children[0].clone();
    Ok(CheckedForwarderProjection {
        facts,
        constructor: dependency,
        call,
    })
}

fn concrete_facts(canonical: &HirFnFacts, haystack: Haystack, result: &str) -> Result<HirFnFacts> {
    let [parameter] = canonical.signature.params.as_slice() else {
        return Err("Checked From requires its exact single formal".into());
    };
    let [call] = canonical.body.children.as_slice() else {
        return Err("Checked From requires its sole constructor call".into());
    };
    if call.kind != "expr_call" || call.children.len() != 1 {
        return Err("Checked From requires its exact single-argument call".into());
    }

    let formal = parameter.pat.hir.clone();
    let normalized = serde_json::to_string(&haystack.token()).map_err(|error| error.to_string())?;
    let mut facts = canonical.clone();
    crate::reference_instances::materialize::normalize_formal(&mut facts, &formal, &normalized)?;
    crate::reference_instances::materialize::normalize_node(&mut facts.body, &formal, &normalized)?;
    normalize_result_sites(&mut facts, result)?;
    Ok(facts)
}

fn normalize_result_sites(facts: &mut HirFnFacts, result: &str) -> Result<()> {
    normalize_result(&mut facts.signature.return_ty.normalized, result)?;
    normalize_result_node(&mut facts.body, result)?;
    normalize_result_node(&mut facts.body.children[0], result)
}

fn normalize_result_node(node: &mut HirNode, result: &str) -> Result<()> {
    let ty = node
        .ty
        .as_mut()
        .ok_or("Checked From loses its result type")?;
    normalize_result(&mut ty.normalized, result)
}

fn normalize_result(normalized: &mut Option<String>, result: &str) -> Result<()> {
    let expected = crate::data_types::parse(result)?;
    if normalized
        .as_deref()
        .map(crate::data_types::parse)
        .transpose()?
        .is_some_and(|captured| captured != expected)
    {
        return Err("Checked From result contradicts its exact constructor dependency".into());
    }
    *normalized = Some(result.into());
    Ok(())
}
