//! Reviewed target-specific compiler identities, never names supplied by a caller.
use super::Definition;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pins {
    target: String,
    core_hash: String,
    stable_crate_id: String,
    definitions: BTreeMap<String, String>,
}

pub(super) fn verify(
    definition: &Definition,
    registry: &crate::data_library::Registry,
    expected: &str,
) -> Result<(), String> {
    let pins = tables()
        .iter()
        .find(|pins| pins.target == registry.target && pins.core_hash == registry.core_hash)
        .ok_or("Source provenance lacks reviewed target-specific core identities")?;
    if definition.local
        || definition.crate_name != "core"
        || definition.crate_hash != pins.core_hash
        || definition.stable_crate_id != pins.stable_crate_id
        || definition.path != expected
        || pins.definitions.get(expected) != Some(&definition.definition_identity)
    {
        return Err("Source provenance disagrees with exact pinned core definition".into());
    }
    Ok(())
}

fn tables() -> &'static [Pins] {
    static PINS: OnceLock<Vec<Pins>> = OnceLock::new();
    PINS.get_or_init(|| {
        serde_json::from_str(include_str!("pins.json"))
            .expect("reviewed bound and assertion compiler identities")
    })
}

pub(super) fn resolved(
    definition: &Definition,
    registry: &crate::data_library::Registry,
    suffix: &str,
) -> Result<(), String> {
    if definition.path != format!("core::{suffix}") && definition.path != format!("std::{suffix}") {
        return Err("Resolved source provenance is not its exact reviewed core/std path".into());
    }
    verify(definition, registry, &definition.path)
}
