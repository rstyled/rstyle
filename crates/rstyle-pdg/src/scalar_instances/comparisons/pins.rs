//! Actual compiler identities, independent of caller-provided observation data.
use super::{Checked, Definition, Observation};
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
    record: &Observation,
    checked: Checked,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    let pins = pins()
        .iter()
        .find(|pins| pins.target == registry.target && pins.core_hash == registry.core_hash)
        .ok_or("Primitive comparison identities are unavailable for this pinned target")?;

    let trait_name = checked.operator.trait_name();
    let method = checked.operator.method();
    let scalar = checked.scalar.name();
    let root = if record.requested.path.starts_with("core::cmp::") {
        "core"
    } else if record.requested.path.starts_with("std::cmp::") {
        "std"
    } else {
        return Err("Primitive comparison has an unreviewed definition path".into());
    };

    let trait_path = format!("{root}::cmp::{trait_name}");
    let impl_path = format!("{root}::cmp::impls::<impl {root}::cmp::{trait_name} for {scalar}>");
    for (definition, path) in [
        (&record.requested_trait, trait_path.clone()),
        (&record.selected_trait, trait_path.clone()),
        (&record.requested, format!("{trait_path}::{method}")),
        (&record.selected_impl, impl_path.clone()),
        (&record.selected, format!("{impl_path}::{method}")),
    ] {
        exact(definition, &path, pins)?;
    }
    Ok(())
}

fn exact(definition: &Definition, path: &str, pins: &Pins) -> Result<(), String> {
    if definition.local
        || definition.crate_name != "core"
        || definition.path != path
        || definition.crate_hash != pins.core_hash
        || definition.stable_crate_id != pins.stable_crate_id
        || pins.definitions.get(path) != Some(&definition.definition_identity)
    {
        return Err(
            "Primitive comparison disagrees with pinned core trait/implementation identity".into(),
        );
    }
    Ok(())
}

fn pins() -> &'static [Pins] {
    static PINS: OnceLock<Vec<Pins>> = OnceLock::new();
    PINS.get_or_init(|| {
        serde_json::from_str(include_str!("pins.json"))
            .expect("reviewed real compiler comparison identities")
    })
}
