//! Reviewed authority for the legacy core Range lang item instantiated at usize.
//!
//! The pinned declaration has precisely two scalar fields and explicitly omits
//! Copy. Review of Drop implementations across the pinned core source finds no
//! Range destructor; downstream crates cannot add Drop to this foreign type.
//! Consequently these two usize fields have no cleanup obligations. Mandatory
//! rustc observations corroborate this contract; none alone grants permission.
//! This proves neither fresh use of a moved value nor any iterator operation.

use crate::{data_library, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

pub const SOURCE_PATH: &str = "ops/range.rs";
pub const SOURCE_DIGEST: &str =
    "sha256:5d791bd2ee8997e93ac72c35db831815d9acbb7aed05cd1539799b7677a13d20";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub definition: data_library::Callee,
    pub lang_item: String,
    pub arguments: Vec<String>,
    pub fields: Vec<Field>,
    pub is_copy: bool,
    pub needs_drop: bool,
    pub has_drop_impl: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub position: usize,
    pub name: String,
    pub scalar: String,
}

/// Verify only the bounded concrete type authority, never whole-function support.
pub fn verify(observation: &Observation, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let facts_ref = facts.facts();
    let registry =
        data_library::registry_for(facts_ref).ok_or("Unpinned Range value compiler/target")?;
    if facts_ref.build.pointer_width.status != crate::FactStatus::Known
        || facts_ref.build.pointer_width.value != Some(64)
        || registry.pointer_width != 64
    {
        return Err("Range values require their reviewed 64-bit target".into());
    }

    data_library::verify_sysroot_roots(facts, &["core", "std"], "Range value")?;
    definition(observation, registry)?;
    shape(observation)
}

fn definition(observation: &Observation, registry: &data_library::Registry) -> Result<(), String> {
    if registry.source_digests.get(SOURCE_PATH).map(String::as_str) != Some(SOURCE_DIGEST) {
        return Err("Missing reviewed Range declaration source authority".into());
    }
    if !["core::ops::Range", "std::ops::Range"].contains(&observation.definition.path.as_str())
        || !registry.exact_core_definition(&observation.definition, "std::ops::Range")
    {
        return Err("Range value disagrees with reviewed core definition".into());
    }
    Ok(())
}

pub(crate) fn shape(observation: &Observation) -> Result<(), String> {
    if observation.lang_item != "Range" || observation.arguments != ["usize"] {
        return Err("Range value requires the exact Range lang item at usize".into());
    }
    if observation.fields.len() != 2
        || observation
            .fields
            .iter()
            .zip(["start", "end"])
            .enumerate()
            .any(|(position, (field, name))| {
                field.position != position || field.name != name || field.scalar != "usize"
            })
    {
        return Err("Range value requires exactly its ordered usize fields".into());
    }
    if observation.is_copy || observation.needs_drop || observation.has_drop_impl {
        return Err("Range value disagrees with reviewed non-Copy/drop-free behavior".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(registry: &data_library::Registry) -> Observation {
        Observation {
            definition: data_library::Callee {
                path: "core::ops::Range".into(),
                crate_name: "core".into(),
                crate_hash: registry.core_hash.clone(),
                definition_hash: registry.definitions["std::ops::Range"].clone(),
                implementation: None,
            },
            lang_item: "Range".into(),
            arguments: vec!["usize".into()],
            fields: ["start", "end"]
                .into_iter()
                .enumerate()
                .map(|(position, name)| Field {
                    position,
                    name: name.into(),
                    scalar: "usize".into(),
                })
                .collect(),
            is_copy: false,
            needs_drop: false,
            has_drop_impl: false,
        }
    }

    #[test]
    fn reviewed_target_definitions_agree_on_concrete_drop_free_shape() {
        assert!(!std::mem::needs_drop::<std::ops::Range<usize>>());
        for registry in data_library::registries() {
            let value = observation(registry);
            definition(&value, registry).unwrap();
            shape(&value).unwrap();
        }
    }

    #[test]
    fn missing_observations_and_unknown_fields_are_not_defaulted() {
        let value = serde_json::to_value(observation(data_library::registry())).unwrap();
        for key in [
            "definition",
            "lang_item",
            "arguments",
            "fields",
            "is_copy",
            "needs_drop",
            "has_drop_impl",
        ] {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<Observation>(missing).is_err());
        }
        let mut extra = value;
        extra["assumed_copy"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Observation>(extra).is_err());
    }

    #[test]
    fn scalar_substitution_field_order_and_cleanup_claims_are_checked() {
        let original = observation(data_library::registry());
        shape(&original).unwrap();
        let mut changed = original.clone();
        changed.arguments[0] = "u64".into();
        assert!(shape(&changed).is_err());
        changed = original.clone();
        changed.fields.swap(0, 1);
        assert!(shape(&changed).is_err());
        for key in ["is_copy", "needs_drop", "has_drop_impl"] {
            let mut wire = serde_json::to_value(&original).unwrap();
            wire[key] = serde_json::json!(true);
            assert!(shape(&serde_json::from_value(wire).unwrap()).is_err());
        }
    }

    #[test]
    fn other_range_names_and_target_definitions_are_unavailable() {
        let registry = data_library::registry();
        let mut value = observation(registry);
        definition(&value, registry).unwrap();
        value.definition.path = "std::range::Range".into();
        assert!(definition(&value, registry).is_err());
        value = observation(&data_library::registries()[1]);
        assert!(definition(&value, registry).is_err());
        value = observation(registry);
        value.definition.implementation = Some(Box::new(value.definition.clone()));
        assert!(definition(&value, registry).is_err());
    }
}
