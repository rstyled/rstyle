//! Independent target and definition tampering expectations.
use super::*;

fn definition(registry: &crate::data_library::Registry, path: &str) -> Definition {
    let rows: serde_json::Value = serde_json::from_str(include_str!("pins.json")).unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["target"] == registry.target && row["core_hash"] == registry.core_hash)
        .unwrap();
    serde_json::from_value(serde_json::json!({
        "path": path,
        "definition_identity": row["definitions"][path],
        "crate": "core",
        "stable_crate_id": row["stable_crate_id"],
        "local": false,
        "crate_hash": row["core_hash"]
    }))
    .unwrap()
}

#[test]
fn resolved_trait_spelling_cannot_replace_actual_core_identity() {
    for registry in registries() {
        let original = definition(registry, "core::cmp::PartialOrd");
        assert!(pins::verify(&original, registry, "core::cmp::PartialOrd").is_ok());
        let changed = [
            Definition {
                definition_identity: format!("{}-unreviewed", original.definition_identity),
                ..original.clone()
            },
            Definition {
                local: true,
                ..original.clone()
            },
            Definition {
                crate_hash: format!("{}0", original.crate_hash),
                ..original.clone()
            },
            Definition {
                stable_crate_id: format!("{}0", original.stable_crate_id),
                ..original
            },
        ];
        for definition in changed {
            assert!(pins::verify(&definition, registry, "core::cmp::PartialOrd").is_err());
        }
    }
}

#[test]
fn trait_identity_cannot_be_transferred_between_targets_or_bounds() {
    let registries = registries();
    for registry in &registries {
        let original = definition(registry, "core::cmp::PartialOrd");
        assert!(pins::verify(&original, registry, "core::cmp::PartialEq").is_err());
        for other in registries
            .iter()
            .filter(|other| other.target != registry.target)
        {
            assert!(pins::verify(&original, other, "core::cmp::PartialOrd").is_err());
        }
    }
}

fn registries() -> Vec<&'static crate::data_library::Registry> {
    let rows: serde_json::Value = serde_json::from_str(include_str!("pins.json")).unwrap();
    let selected: Vec<_> = crate::data_library::registries()
        .iter()
        .filter(|registry| {
            rows.as_array().unwrap().iter().any(|row| {
                row["target"] == registry.target && row["core_hash"] == registry.core_hash
            })
        })
        .collect();
    assert_eq!(selected.len(), 3);
    selected
}

#[test]
fn unavailable_compiler_target_registry_has_no_provenance_authority() {
    let original = definition(registries()[0], "core::cmp::PartialOrd");
    let mut unknown: crate::data_library::Registry =
        serde_json::from_str(include_str!("../../../contracts/read-only-lengths-v2.json")).unwrap();
    unknown.target = "unreviewed-target".into();
    assert!(pins::verify(&original, &unknown, "core::cmp::PartialOrd").is_err());
}
