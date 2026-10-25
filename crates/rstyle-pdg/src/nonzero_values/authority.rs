//! Reviewed rustc/core identities. Foreign targets need their own independent pins.
use super::{Definition, Observation, Scalar};
use crate::unit_store::EvidenceFacts;

pub(super) const NONZERO: (&str, &str) = (
    "core::num::NonZero",
    "DefPathHash(Fingerprint(5614226571623188065, 17233351565658270785))",
);
pub(super) const NEW: (&str, &str) = (
    "core::num::NonZero::<T>::new",
    "DefPathHash(Fingerprint(5614226571623188065, 15673364348431969571))",
);
pub(super) const OPTION: (&str, &str) = (
    "core::option::Option",
    "DefPathHash(Fingerprint(5614226571623188065, 5328416817104062757))",
);

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts.facts())
        .ok_or("Unpinned NonZero compiler/target")?;
    if registry.target != "aarch64-unknown-linux-gnu"
        || registry.rustc_commit != "5c543b0b8c73c7b72bc8284ced4fb22ead15734d"
        || registry.core_hash != "bdb4eda0487b663ec13960d84fc76cc8"
        || facts.facts().build.pointer_width.status != crate::FactStatus::Known
        || facts.facts().build.pointer_width.value != Some(64)
    {
        return Err("NonZero requires reviewed native core build authority".into());
    }
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "NonZero")
}

pub(super) fn identity((path, hash): (&str, &str)) -> Definition {
    Definition {
        path: path.into(),
        definition_identity: hash.into(),
        crate_name: "core".into(),
        stable_crate_id: "StableCrateId(5614226571623188065)".into(),
        local: false,
    }
}

pub(super) fn definition(observed: &Definition, expected: (&str, &str)) -> Result<(), String> {
    let mut reviewed = identity(expected);
    if observed.path == expected.0.replacen("core::", "std::", 1) {
        reviewed.path.clone_from(&observed.path);
    }
    if observed != &reviewed {
        return Err("NonZero definition differs from reviewed core authority".into());
    }
    Ok(())
}

pub(super) fn observation(scalar: Scalar) -> Observation {
    Observation {
        definition: identity(NONZERO),
        scalar,
    }
}
