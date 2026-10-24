//! Exact concrete Default source/call association; ordinary bodies prove effects.
mod model;
mod selection;
mod source;
#[cfg(test)]
mod tests;

use crate::{
    HirFnFacts,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
};
use model::Definition;

pub(crate) const CAPABILITY: &str = "rust.checked-concrete-default";
pub(crate) const KEY: &str = "rstyle.rust.concrete-default-member";
pub(crate) use selection::verify as verify_selection;
pub(crate) use source::{SourceMember, member as source_member};

fn observation(facts: &HirFnFacts) -> Result<Option<model::Observation>, String> {
    let Some(value) = facts.build.frontend_options.get(KEY) else {
        return Ok(None);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Concrete Default requires its registered capability".into());
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
    let outcome: Outcome =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    let observation = match outcome {
        Outcome::Observed { observation } => observation,
        Outcome::Unsupported { reason } => {
            return Err(format!(
                "Unsupported concrete Default observation: {reason}"
            ));
        }
    };
    if observation.identity != "rstyle.rust.concrete-default-member-observation/1"
        || observation.admission_authority
    {
        return Err("Default owning observation has another identity or authority".into());
    }

    authority(
        &observation.trait_definition,
        facts,
        "rust.concrete-default.trait",
    )?;
    authority(
        &observation.trait_method,
        facts,
        "rust.concrete-default.method",
    )?;
    Ok(Some(*observation))
}

fn authority(definition: &Definition, facts: &HirFnFacts, key: &str) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned Default compiler/target")?;
    let names: &[&str] = if key == "rust.concrete-default.trait" {
        &["core::default::Default", "std::default::Default"]
    } else {
        &[
            "core::default::Default::default",
            "std::default::Default::default",
        ]
    };
    if definition.local
        || definition.crate_name != "core"
        || registry.definitions.get("rust.concrete-default.core-crate")
            != Some(&definition.stable_crate_id)
        || !names.contains(&definition.path.as_str())
        || registry.definitions.get(key) != Some(&definition.definition_identity)
    {
        return Err("Default trait/member differs from reviewed core authority".into());
    }
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Concrete Default")
}

fn result(facts: &HirFnFacts) -> Result<TypeTree, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured { ty } = &interface.result else {
        return Err("Default requires its complete concrete result".into());
    };
    let TypeTree::Struct { .. } = ty else {
        return Err("Default requires a concrete product result".into());
    };

    if !crate::data_aggregates::reference_free(&crate::data_types::TypeToken::from_tree(ty)) {
        return Err("Default result must be a complete reference-free product".into());
    }
    Ok(ty.clone())
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        CAPABILITY,
        || {
            facts.build.frontend_options.contains_key(KEY)
                || crate::helper_calls::call_nodes(facts).iter().any(|node| {
                    node.res.as_ref().is_some_and(|resolution| {
                        serde_json::from_str::<serde_json::Value>(&resolution.description)
                            .is_ok_and(|record| record.get("default_selection").is_some())
                    })
                })
        },
        "Concrete Default requires its registered document policy",
    )
}
