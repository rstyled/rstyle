//! Reviewed forwarding and reflexive core bodies, separate from observed selection.
use super::model::{Definition, Observation, Selection};
use crate::{HirFnFacts, HirNode, data_library::Registry};

mod range;
pub(super) use range::local_definition;
pub(super) use range::verify as verify_range;

const SOURCE: &str = "sha256:ec1dfed5b8eef22496fc85eeb0b26a5600944fb8d2f6d6e9aaf81fb45af06de5";

pub(super) fn verify(
    canonical: &HirFnFacts,
    observed: &Observation,
    call: &HirNode,
) -> Result<(), String> {
    let registry = reviewed(canonical)?;
    definitions(registry, observed)?;
    original_call(observed, call)
}

fn reviewed(canonical: &HirFnFacts) -> Result<&'static Registry, String> {
    let registry = crate::data_library::registry_for(canonical)
        .ok_or("Unpinned product conversion compiler/target")?;
    crate::data_library::verify_sysroot_roots(canonical, &["core", "std"], "Product conversion")?;
    if registry
        .result_propagation_source_digests
        .get("convert/mod.rs")
        .map(String::as_str)
        != Some(SOURCE)
    {
        return Err(
            "Product conversion lacks reviewed complete core conversion source authority".into(),
        );
    }
    super::review::verify()?;

    Ok(registry)
}

fn original_call(observed: &Observation, call: &HirNode) -> Result<(), String> {
    let requested = crate::data_library::callee(call)?;
    if requested != observed.into.requested.callee()? {
        return Err("Product original call differs from its pinned requested Into method".into());
    }
    Ok(())
}

type Pin = (&'static str, &'static str);
const INTO: &[Pin] = &[
    (
        "rust.product-conversion.into-method",
        "core::convert::Into::into",
    ),
    ("rust.product-conversion.into-trait", "core::convert::Into"),
    ("rust.product-conversion.into-trait", "core::convert::Into"),
    (
        "rust.product-conversion.into-blanket-method",
        "<T as core::convert::Into<U>>::into",
    ),
    (
        "rust.product-conversion.into-blanket-impl",
        "<T as core::convert::Into<U>>",
    ),
];
const FROM: &[Pin] = &[
    (
        "rstyle.result-propagation.identity-from.method",
        "core::convert::From::from",
    ),
    ("rust.product-conversion.from-trait", "core::convert::From"),
    ("rust.product-conversion.from-trait", "core::convert::From"),
    (
        "rstyle.result-propagation.identity-from.implementation",
        "<T as core::convert::From<T>>::from",
    ),
    (
        "rust.product-conversion.identity-from-impl",
        "<T as core::convert::From<T>>",
    ),
];

fn definitions(registry: &Registry, observed: &Observation) -> Result<(), String> {
    let stable = &observed.into.requested.stable_crate_id;
    selection(registry, &observed.into, INTO, stable)?;
    selection(registry, &observed.from, FROM, stable)
}

fn selection(
    registry: &Registry,
    observed: &Selection,
    pins: &[Pin],
    stable: &str,
) -> Result<(), String> {
    let definitions = [
        &observed.requested,
        &observed.requested_trait,
        &observed.selected_trait,
        &observed.selected,
        &observed.selected_impl,
    ];
    if pins.len() != definitions.len() {
        return Err("Incomplete reviewed product conversion pin roster".into());
    }

    for (definition, (key, path)) in definitions.into_iter().zip(pins) {
        pinned(registry, definition, key, path)?;
        if definition.stable_crate_id != stable {
            return Err("Product conversion crosses observed core crate identities".into());
        }
    }
    Ok(())
}

fn pinned(
    registry: &Registry,
    definition: &Definition,
    key: &str,
    path: &str,
) -> Result<(), String> {
    if (definition.path != path && definition.path != path.replace("core::", "std::"))
        || !registry.exact_core_definition(&definition.callee()?, key)
    {
        return Err(format!(
            "Product conversion lacks exact reviewed definition {key}"
        ));
    }
    Ok(())
}

/// Definition ownership is independent of whether a local item is a type or method.
pub(super) fn local_compiled(
    canonical: &HirFnFacts,
    definition: &Definition,
    missing: &str,
) -> Result<bool, String> {
    let compiled = canonical
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or(missing)?;
    Ok(definition.local
        && definition.crate_hash.is_none()
        && !definition.definition_identity.is_empty()
        && !definition.path.is_empty()
        && compiled["identity"] == "rstyle.rust.compiled-crate/1"
        && compiled["crate_name"].as_str() == Some(definition.crate_name.as_str())
        && compiled["stable_crate_id"].as_str() == Some(definition.stable_crate_id.as_str()))
}
