//! Original concrete TryFrom association. This leaf proves no callable effects,
//! Copy, cleanup or lexical namespace permission; the ordinary pipeline does.
mod authority;
pub(crate) mod input;
pub(crate) mod model;
mod owner;
mod selection;
mod source;
pub(crate) mod syntax;
#[cfg(test)]
mod tests;

use crate::{HirFnFacts, concrete_types::TypeTree};
pub(crate) use authority::RegistryAuthority;
pub(crate) use model::Definition;
pub(crate) use owner::{
    core as verify_core, endpoint, local as verify_local, nominal as verify_nominal,
    structural_signature as verify_signature,
};
pub(crate) use selection::{Association, SelectionInput, association, verify_selection};
pub(crate) use source::{SourceMember, source_member, verify_owner};

pub(crate) const KEY: &str = "rstyle.rust.concrete-conversion-member";
pub(crate) const CAPABILITY: &str = "rust.checked-concrete-tryfrom";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoreRole {
    Trait,
    Method,
    ErrorMember,
    Result,
}

/// An obligatory independently reviewed core identity/build/source verifier.
/// Matching names alone cannot implement this contract. Parent integration must
/// pin the actual trait/member/associated Error/Result, compiler/target, stable
/// crate identity, source contracts and sysroot roots for each supported build.
pub(crate) trait CoreAuthority {
    fn verify(
        &self,
        facts: &HirFnFacts,
        role: CoreRole,
        definition: &Definition,
    ) -> Result<(), String>;
}

pub(super) fn result(facts: &HirFnFacts) -> Result<(TypeTree, TypeTree), String> {
    use crate::concrete_types::{InterfaceObservations, Observation};
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured {
        ty: TypeTree::Result { ok, err },
    } = &interface.result
    else {
        return Err("Concrete TryFrom requires a complete captured Result return".into());
    };
    if !crate::concrete_results::scalar_errors::GRAMMAR.permits(
        &crate::data_types::TypeToken::from_tree(ok),
        &crate::data_types::TypeToken::from_tree(err),
    ) {
        return Err("Concrete TryFrom requires its bounded scalar product/error shape".into());
    }

    Ok((ok.as_ref().clone(), err.as_ref().clone()))
}

fn observation(facts: &HirFnFacts) -> Result<Option<model::Observation>, String> {
    let Some(value) = facts.build.frontend_options.get(KEY) else {
        return Ok(None);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Concrete TryFrom requires its registered capability".into());
    }
    #[derive(serde::Deserialize)]
    #[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
    enum Outcome {
        Observed {
            observation: Box<model::Observation>,
        },
        Unsupported {
            reason: String,
        },
    }
    let outcome: Outcome = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    let observation = match outcome {
        Outcome::Observed { observation } => *observation,
        Outcome::Unsupported { reason } => {
            return Err(format!("Unsupported concrete TryFrom: {reason}"));
        }
    };
    if observation.identity != "rstyle.rust.concrete-conversion-member-observation/1"
        || observation.admission_authority
    {
        return Err("Concrete TryFrom observation has another identity or claims admission".into());
    }

    Ok(Some(observation))
}

/// Original owning member proof only; surrounding helper validation supplies effects.
pub(crate) fn verified_member(facts: &HirFnFacts) -> Result<model::Observation, String> {
    let observation = observation(facts)?.ok_or("Missing selected TryFrom owner")?;
    owner::verify(facts, &observation, &RegistryAuthority)?;
    Ok(observation)
}
