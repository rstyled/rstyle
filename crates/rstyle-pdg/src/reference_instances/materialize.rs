//! Private concrete facts derived from the complete original constructor proof.
//! Source findings and generic spellings remain available to frontend gates.
#[cfg(test)]
mod tests;

use crate::{HirFnFacts, HirNode, HirRef, data_types::TypeToken};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Haystack {
    Str,
    Bytes,
}

impl Haystack {
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Str => "str",
            Self::Bytes => "[u8]",
        }
    }

    pub(in crate::reference_instances) fn token(self) -> TypeToken {
        let referent = match self {
            Self::Str => TypeToken::Str,
            Self::Bytes => TypeToken::Slice(Box::new(TypeToken::Scalar("u8".into()))),
        };
        TypeToken::Shared(Box::new(referent))
    }
}

/// Cannot be constructed from serialized facts or an observation alone.
pub struct CheckedProjection {
    facts: HirFnFacts,
    haystack: Haystack,
    as_ref: HirNode,
    products: super::products::Products,
}

impl CheckedProjection {
    pub fn facts(&self) -> &HirFnFacts {
        &self.facts
    }

    pub fn haystack(&self) -> Haystack {
        self.haystack
    }

    pub(crate) fn products(&self) -> &super::products::Products {
        &self.products
    }

    pub(super) fn as_ref(&self) -> &HirNode {
        &self.as_ref
    }
}

pub fn verify(
    canonical: &HirFnFacts,
    observation: &Value,
    haystack: Haystack,
) -> Result<CheckedProjection, String> {
    super::verify_constructor_body_observations(canonical, observation, haystack.spelling())?;
    crate::nominal_boundary::resolution::probe_reference_product_result(canonical)?;

    let facts = concrete_facts(canonical, haystack)?;
    let products = super::products::Products::checked(&facts)?;
    let as_ref = facts
        .body
        .children
        .first()
        .and_then(|statement| statement.children.get(1))
        .ok_or("Checked constructor lost its exact saved AsRef operation")?
        .clone();

    Ok(CheckedProjection {
        facts,
        haystack,
        as_ref,
        products,
    })
}

fn concrete_facts(canonical: &HirFnFacts, haystack: Haystack) -> Result<HirFnFacts, String> {
    let [parameter] = canonical.signature.params.as_slice() else {
        return Err("Reference constructor requires its exact single formal".into());
    };
    let formal = parameter.pat.hir.clone();
    let normalized = serde_json::to_string(&haystack.token()).map_err(|error| error.to_string())?;

    let mut facts = canonical.clone();
    normalize_formal(&mut facts, &formal, &normalized)?;
    normalize_node(&mut facts.body, &formal, &normalized)?;
    Ok(facts)
}

pub(in crate::reference_instances) fn normalize_formal(
    facts: &mut HirFnFacts,
    formal: &HirRef,
    normalized: &str,
) -> Result<(), String> {
    facts.signature.params[0].ty.normalized = Some(normalized.into());
    normalize_node(&mut facts.signature.params[0].pat, formal, normalized)?;
    for binding in &mut facts.bindings {
        if binding.hir == *formal {
            binding.ty.normalized = Some(normalized.into());
        }
    }
    Ok(())
}

pub(in crate::reference_instances) fn normalize_node(
    node: &mut HirNode,
    formal: &HirRef,
    normalized: &str,
) -> Result<(), String> {
    let formal_read = node.res.as_ref().is_some_and(|resolution| {
        resolution.kind == "local" && resolution.target.as_ref() == Some(formal)
    });
    if &node.hir == formal || formal_read {
        normalize_type(node, normalized)?;
    }
    for child in &mut node.children {
        normalize_node(child, formal, normalized)?;
    }
    Ok(())
}

fn normalize_type(node: &mut HirNode, normalized: &str) -> Result<(), String> {
    let ty = node
        .ty
        .as_mut()
        .ok_or("Reference formal lost its captured type")?;
    ty.normalized = Some(normalized.into());
    Ok(())
}
