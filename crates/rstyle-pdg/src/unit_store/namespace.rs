//! Exported namespaces qualify local handles; they do not fingerprint contents.
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

/// One local producer process and allocation ordinal. Independently loaded
/// copies retain the wire namespace but own separate views and proof caches.
/// These values are neither authentication nor globally persistent identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitNamespace {
    process: u32,
    epoch_nanos: u64,
    ordinal: u64,
}

impl UnitNamespace {
    pub(super) fn fresh() -> Result<Self, String> {
        static EPOCH: OnceLock<Result<u64, String>> = OnceLock::new();
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let epoch = EPOCH
            .get_or_init(|| {
                let elapsed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| error.to_string())?;
                u64::try_from(elapsed.as_nanos())
                    .map_err(|_| "Producer epoch exceeds namespace range".into())
            })
            .clone()?;

        let ordinal = NEXT
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| "Unit namespace allocation exhausted")?;
        Ok(Self {
            process: std::process::id(),
            epoch_nanos: epoch,
            ordinal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_allocations_have_distinct_namespaces() {
        let first = UnitNamespace::fresh().unwrap();
        let second = UnitNamespace::fresh().unwrap();
        assert_ne!(first, second);
        assert_eq!(first.process, second.process);
        assert_eq!(first.epoch_nanos, second.epoch_nanos);
        assert!(first.ordinal < second.ordinal);
        let wire = serde_json::to_vec(&first).unwrap();
        assert_eq!(first, serde_json::from_slice(&wire).unwrap());
        assert!(serde_json::from_str::<UnitNamespace>("{}").is_err());
    }
}

#[cfg(test)]
mod store_tests {
    use super::super::UnitStore;

    #[test]
    fn namespaces_are_allocated_without_fingerprinting_and_survive_import() {
        let first = UnitStore::capture(Vec::new()).unwrap();
        let second = UnitStore::capture(Vec::new()).unwrap();
        assert_ne!(first.namespace(), second.namespace());
        let mut wire = Vec::new();
        first.write_bundle(&mut wire).unwrap();
        let imported = UnitStore::from_bundle(&wire).unwrap();
        assert_eq!(first.namespace(), imported.namespace());
        let mut document: serde_json::Value = serde_json::from_slice(&wire).unwrap();
        assert_eq!(document["schema"], "rstyle.shared-unit/2");
        assert!(document.get("identity").is_none());
        document["identity"] = "retired fingerprint".into();
        assert!(UnitStore::from_bundle(&serde_json::to_vec(&document).unwrap()).is_err());
        document.as_object_mut().unwrap().remove("identity");
        document["schema"] = "rstyle.shared-unit/1".into();
        assert!(UnitStore::from_bundle(&serde_json::to_vec(&document).unwrap()).is_err());
    }
}
