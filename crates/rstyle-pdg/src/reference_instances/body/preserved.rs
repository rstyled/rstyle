//! Bind early diagnostic facts to the full canonical owner without replacing it.
mod shape;

use super::{Result, model::Observation};
use crate::{BuildFacts, HirFnFacts};
use serde_json::Value;

const ORIGINAL_OPTIONS: [&str; 9] = [
    "definition_identity",
    "edition",
    "extraction_profile",
    "invocation",
    "rstyle.nominal_selected_unit",
    "rstyle.rust.compiled-crate",
    "source_file",
    "source_identifier",
    "target",
];

pub(super) fn verify(
    canonical: &HirFnFacts,
    record: &Observation,
    raw: &Value,
    haystack: &str,
) -> Result<()> {
    if canonical.name != "util::search::Input::<'h>::new"
        || record.status != "observed"
        || record.request.function != canonical.name
        || record.request.haystack != haystack
        || !matches!(haystack, "str" | "[u8]")
        || !record.calls.is_empty()
    {
        return Err(
            "Constructor body proof requires its exact observed Input::new instance".into(),
        );
    }
    let original = &record.original;
    if original.name != canonical.name
        || original.fn_span != canonical.fn_span
        || original.source != canonical.source
        || settings(&original.build) != settings(&canonical.build)
    {
        return Err(
            "Reference diagnostic changed complete canonical source/HIR/build evidence".into(),
        );
    }

    shape::signature(&original.signature, &canonical.signature)?;
    shape::node(&original.body, &canonical.body)?;
    bindings(original, canonical)?;
    // function_source captures before source scanning. Its explicit empty census
    // must never replace or clear the canonical post-scan diagnostics.
    if original.source_scan_complete || !original.source_diagnostics.is_empty() {
        return Err("Reference diagnostic is not the declared pre-scan source observation".into());
    }
    options(canonical, &raw["original"]["build"]["frontend_options"])?;
    definition(canonical, &record.definition)?;
    super::super::provenance::verify(canonical, &record.provenance, haystack)
}

fn options(canonical: &HirFnFacts, raw: &Value) -> Result<()> {
    let options = raw
        .as_object()
        .ok_or("Missing original frontend option roster")?;
    let expected = &ORIGINAL_OPTIONS;
    if options.len() != expected.len() || expected.iter().any(|key| !options.contains_key(*key)) {
        return Err("Reference diagnostic original frontend option roster changed".into());
    }

    for (key, value) in options {
        if canonical.build.frontend_options.get(key) != Some(value) {
            return Err("Reference diagnostic original option differs from canonical owner".into());
        }
    }
    Ok(())
}

fn definition(canonical: &HirFnFacts, observed: &super::model::Definition) -> Result<()> {
    let options = &canonical.build.frontend_options;
    let owner = options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Missing canonical compiled owner")?;
    if !observed.local
        || observed.path != canonical.name
        || options.get("definition_identity").and_then(Value::as_str)
            != Some(observed.definition_identity.as_str())
        || owner.get("crate_name").and_then(Value::as_str) != Some(observed.crate_name.as_str())
        || owner.get("stable_crate_id").and_then(Value::as_str)
            != Some(observed.stable_crate_id.as_str())
    {
        return Err("Constructor observation is detached from its canonical compiler owner".into());
    }
    Ok(())
}

fn settings(build: &BuildFacts) -> BuildFacts<()> {
    BuildFacts {
        frontend_options: (),
        producer_revision: build.producer_revision.clone(),
        rustc_version: build.rustc_version.clone(),
        rustc_commit: build.rustc_commit.clone(),
        edition: build.edition.clone(),
        target_triple: build.target_triple.clone(),
        pointer_width: build.pointer_width.clone(),
        data_layout: build.data_layout.clone(),
        cfgs: build.cfgs.clone(),
        features: build.features.clone(),
        overflow_checks: build.overflow_checks.clone(),
        debug_assertions: build.debug_assertions.clone(),
        panic_strategy: build.panic_strategy.clone(),
        opt_level: build.opt_level.clone(),
    }
}

fn bindings(original: &HirFnFacts, canonical: &HirFnFacts) -> Result<()> {
    if original.bindings.len() != canonical.bindings.len() {
        return Err("Constructor diagnostic changes complete binding inventory".into());
    }
    for (before, after) in original.bindings.iter().zip(&canonical.bindings) {
        if (
            &before.symbol,
            &before.name,
            before.decl_span,
            &before.scope,
            &before.hir,
            before.is_param,
            before.is_mut,
        ) != (
            &after.symbol,
            &after.name,
            after.decl_span,
            &after.scope,
            &after.hir,
            after.is_param,
            after.is_mut,
        ) {
            return Err("Constructor diagnostic changes original binding identity".into());
        }
        shape::ty(&before.ty, &after.ty)?;
    }
    Ok(())
}
