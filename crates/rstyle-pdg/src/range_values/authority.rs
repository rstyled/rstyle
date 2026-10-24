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
    let registry = data_library::verify_64bit_sysroot(facts, "Range value")?;
    definition(observation, registry)?;
    shape(observation)
}

fn definition(observation: &Observation, registry: &data_library::Registry) -> Result<(), String> {
    definition_for(observation, registry, "Range")
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

/// Exact RangeTo observation for checked signatures, never standalone value permission.
pub fn verify_range_to(observation: &Observation, facts: &dyn EvidenceFacts) -> Result<(), String> {
    verify_one_ended(observation, facts, Carrier::Prefix)
}

/// Non-Copy RangeFrom is only a fresh checked carrier, never stored or reused.
pub fn verify_range_from(
    observation: &Observation,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    verify_one_ended(observation, facts, Carrier::Suffix)
}

#[derive(Clone, Copy)]
enum Carrier {
    Prefix,
    Suffix,
}

impl Carrier {
    fn contract(self) -> (&'static str, &'static str, bool, &'static str) {
        match self {
            Self::Prefix => ("RangeTo", "end", true, "rust.checked-borrowed-result-flow"),
            Self::Suffix => (
                "RangeFrom",
                "start",
                false,
                "rust.checked-suffix-result-views",
            ),
        }
    }
}

fn verify_one_ended(
    observation: &Observation,
    facts: &dyn EvidenceFacts,
    carrier: Carrier,
) -> Result<(), String> {
    let (name, field, is_copy, capability) = carrier.contract();
    if !crate::extraction_profile::facts_support(facts, capability) {
        return Err("One-ended range observation requires its checked-access capability".into());
    }
    let registry = data_library::registry_for(facts.facts())
        .ok_or("Unpinned checked range compiler/target")?;
    if facts.facts().build.pointer_width.status != crate::FactStatus::Known
        || facts.facts().build.pointer_width.value != Some(64)
        || registry.pointer_width != 64
    {
        return Err("Checked range observation requires its reviewed 64-bit target".into());
    }

    data_library::verify_sysroot_roots(facts, &["core", "std"], "Checked range observation")?;
    definition_for(observation, registry, name)?;
    one_ended_shape(observation, name, field, is_copy)
}

fn definition_for(
    observation: &Observation,
    registry: &data_library::Registry,
    name: &str,
) -> Result<(), String> {
    let core = format!("core::ops::{name}");
    let std = format!("std::ops::{name}");
    if registry.source_digests.get(SOURCE_PATH).map(String::as_str) != Some(SOURCE_DIGEST)
        || ![core.as_str(), std.as_str()].contains(&observation.definition.path.as_str())
        || !registry.exact_core_definition(&observation.definition, &std)
    {
        return Err("Checked range observation lacks exact pinned definition/source".into());
    }
    Ok(())
}

#[cfg(test)]
fn range_to_definition(
    observation: &Observation,
    registry: &data_library::Registry,
) -> Result<(), String> {
    definition_for(observation, registry, "RangeTo")
}

pub(crate) fn range_to_shape(observation: &Observation) -> Result<(), String> {
    one_ended_shape(observation, "RangeTo", "end", true)
}

pub(crate) fn range_from_shape(observation: &Observation) -> Result<(), String> {
    one_ended_shape(observation, "RangeFrom", "start", false)
}

fn one_ended_shape(
    observation: &Observation,
    name: &str,
    field: &str,
    is_copy: bool,
) -> Result<(), String> {
    if observation.lang_item != name
        || observation.arguments != ["usize"]
        || observation.fields
            != [Field {
                position: 0,
                name: field.into(),
                scalar: "usize".into(),
            }]
        || observation.is_copy != is_copy
        || observation.needs_drop
        || observation.has_drop_impl
    {
        return Err(
            "Checked range observation contradicts exact field/Copy/cleanup evidence".into(),
        );
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

    fn range_to_observation(registry: &data_library::Registry) -> Observation {
        let mut value = observation(registry);
        value.definition.path = "core::ops::RangeTo".into();
        value.definition.definition_hash = registry.definitions["std::ops::RangeTo"].clone();
        value.lang_item = "RangeTo".into();
        value.fields = vec![Field {
            position: 0,
            name: "end".into(),
            scalar: "usize".into(),
        }];
        value.is_copy = true;
        value
    }

    #[test]
    fn suffix_carrier_is_non_copy_and_requires_exact_start_and_source_identity() {
        assert!(!std::mem::needs_drop::<std::ops::RangeFrom<usize>>());
        for registry in data_library::registries() {
            let mut value = observation(registry);
            value.definition.path = "core::ops::RangeFrom".into();
            value.definition.definition_hash = registry.definitions["std::ops::RangeFrom"].clone();
            value.lang_item = "RangeFrom".into();
            value.fields.truncate(1);
            definition_for(&value, registry, "RangeFrom").unwrap();
            range_from_shape(&value).unwrap();
            assert!(range_to_shape(&value).is_err());
            for mutation in 0..10 {
                let mut changed = value.clone();
                match mutation {
                    0 => changed.lang_item = "RangeTo".into(),
                    1 => changed.arguments = vec!["u64".into()],
                    2 => changed.fields.clear(),
                    3 => changed.fields[0].name = "end".into(),
                    4 => changed.fields[0].scalar = "u64".into(),
                    5 => changed.fields[0].position = 1,
                    6 => changed.is_copy = true,
                    7 => changed.needs_drop = true,
                    8 => changed.has_drop_impl = true,
                    _ => changed.fields.push(changed.fields[0].clone()),
                }
                assert!(range_from_shape(&changed).is_err());
            }
            value.definition.definition_hash = registry.definitions["std::ops::RangeTo"].clone();
            assert!(definition_for(&value, registry, "RangeFrom").is_err());
        }
    }

    #[test]
    fn range_to_requires_independent_complete_shape_and_exact_target_authority() {
        for registry in data_library::registries() {
            let value = range_to_observation(registry);
            range_to_definition(&value, registry).unwrap();
            range_to_shape(&value).unwrap();
            assert!(shape(&value).is_err());
            for mutation in 0..9 {
                let mut changed = value.clone();
                match mutation {
                    0 => changed.lang_item = "Range".into(),
                    1 => changed.arguments.clear(),
                    2 => changed.fields.clear(),
                    3 => changed.fields[0].name = "start".into(),
                    4 => changed.fields[0].scalar = "u64".into(),
                    5 => changed.fields[0].position = 1,
                    6 => changed.is_copy = false,
                    7 => changed.needs_drop = true,
                    _ => changed.has_drop_impl = true,
                }
                assert!(range_to_shape(&changed).is_err());
            }
            let mut wrong_definition = value;
            wrong_definition.definition.definition_hash = "forged".into();
            assert!(range_to_definition(&wrong_definition, registry).is_err());
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
