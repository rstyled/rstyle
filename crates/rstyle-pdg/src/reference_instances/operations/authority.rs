//! Reviewed native core/source identities, separate from observed selections.
use super::{Definition, Observation};
use crate::HirFnFacts;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, sync::OnceLock};

mod review;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pins {
    rustc_commit: String,
    target: String,
    core_hash: String,
    stable_crate_id: String,
    definitions: BTreeMap<String, String>,
}

pub(in crate::reference_instances) fn bound(
    canonical: &HirFnFacts,
    definition: &Value,
    expected: &str,
) -> Result<(), String> {
    let definition: Definition =
        serde_json::from_value(definition.clone()).map_err(|e| e.to_string())?;
    check(canonical, &definition, expected)
}

pub(super) fn operations(
    canonical: &HirFnFacts,
    observed: &Observation,
    haystack: &str,
    call: &crate::HirNode,
) -> Result<(), String> {
    check(
        canonical,
        &observed.requested,
        "core::convert::AsRef::as_ref",
    )?;
    check(canonical, &observed.requested_trait, "core::convert::AsRef")?;
    check(canonical, &observed.selected_trait, "core::convert::AsRef")?;

    let (implementation, method) = match haystack {
        "[u8]" => (
            "<[T] as core::convert::AsRef<[T]>>",
            "<[T] as core::convert::AsRef<[T]>>::as_ref",
        ),
        "str" => (
            "core::str::<impl core::convert::AsRef<[u8]> for str>",
            "core::str::<impl core::convert::AsRef<[u8]> for str>::as_ref",
        ),
        _ => return Err("AsRef authority has no reviewed instance contract".into()),
    };
    check(canonical, &observed.selected_impl, implementation)?;
    check(canonical, &observed.selected, method)?;

    let callee = crate::data_library::callee(call)?;
    if callee.path != observed.requested.path
        || callee.crate_name != observed.requested.crate_name
        || callee.crate_hash != observed.requested.crate_hash
        || callee.definition_hash != observed.requested.definition_identity
        || callee.implementation.is_some()
    {
        return Err(
            "AsRef original callee differs from independently checked requested definition".into(),
        );
    }
    Ok(())
}

fn check(canonical: &HirFnFacts, definition: &Definition, expected: &str) -> Result<(), String> {
    let pins = pins();
    let registry =
        crate::data_library::registry_for(canonical).ok_or("Unpinned AsRef compiler/target")?;
    if canonical.build.rustc_commit.known_value() != Some(pins.rustc_commit.as_str())
        || registry.target != pins.target
        || registry.core_hash != pins.core_hash
    {
        return Err("AsRef source authority is unavailable for this compiler/target".into());
    }

    crate::data_library::verify_sysroot_roots(canonical, &["core", "std"], "AsRef")?;
    if definition.local
        || definition.crate_name != "core"
        || definition.crate_hash != pins.core_hash
        || definition.stable_crate_id != pins.stable_crate_id
        || definition.path != expected
        || pins.definitions.get(expected) != Some(&definition.definition_identity)
    {
        return Err(
            "AsRef definition differs from reviewed core source/implementation authority".into(),
        );
    }

    static REVIEW: OnceLock<Result<(), String>> = OnceLock::new();
    REVIEW.get_or_init(review::verify).clone()
}

fn pins() -> &'static Pins {
    static PINS: OnceLock<Pins> = OnceLock::new();
    PINS.get_or_init(|| {
        serde_json::from_str(include_str!("authority/pins.json"))
            .expect("reviewed native reference-instance identities")
    })
}
