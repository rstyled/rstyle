//! Independent checks of selected Input constructor conversion observations.
//! Success grants no caller, cleanup, effect, returned-origin or admission authority.
use super::view_types::{BorrowedViewType, ViewType};
use crate::{HirFnFacts, HirNode};
use serde::Deserialize;
use serde_json::Value;

pub(super) mod authority;
mod source;
#[cfg(test)]
mod tests;
mod types;

use types::{Argument, Definition, Signature, Type};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    identity: String,
    hir: crate::HirRef,
    span: crate::ByteSpan,
    expanded: bool,
    requested: Definition,
    requested_trait: Definition,
    original_arguments: Vec<Argument>,
    requested_arguments: Vec<Argument>,
    selected: Definition,
    selected_arguments: Vec<Argument>,
    selected_impl: Definition,
    selected_trait: Definition,
    requested_signature: Signature,
    signature: Signature,
    receiver_adjustments: Adjustments,
    receiver_type: Vec<Argument>,
    result_type: Vec<Argument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Adjustments {
    hir: crate::HirRef,
    span: crate::ByteSpan,
    source_type: Vec<Argument>,
    steps: Vec<Step>,
    adjusted_type: Vec<Argument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    kind: String,
    target: Vec<Argument>,
}

pub(super) fn verify(
    canonical: &HirFnFacts,
    conversions: &[Value],
    haystack: &str,
) -> Result<(), String> {
    if crate::extraction_profile::ExtractionProfile::from_facts(canonical)?
        != crate::extraction_profile::ExtractionProfile::DataV3
    {
        return Err("AsRef instance observations require their original DataV3 owner".into());
    }
    let [conversion] = conversions else {
        return Err("Input constructor requires exactly one complete AsRef observation".into());
    };
    let observation: Observation =
        serde_json::from_value(conversion.clone()).map_err(|e| e.to_string())?;
    let (call, receiver) = source::site(canonical)?;
    if observation.identity != "rstyle.rust.borrowed-as-ref-observation/1"
        || observation.hir != call.hir
        || observation.span != call.span
        || observation.expanded
    {
        return Err("AsRef observation differs from its exact original call site".into());
    }

    let source = Type::haystack(haystack)?;
    authority::operations(canonical, &observation, haystack, call)?;
    structural(&observation, &source)?;
    adjustments(&observation.receiver_adjustments, receiver, &source)
}

fn structural(observation: &Observation, source: &Type) -> Result<(), String> {
    let [original_self, original_target] = observation.original_arguments.as_slice() else {
        return Err("AsRef original generic argument roster disagrees".into());
    };
    original_self.original_generic()?;
    original_target.verify(&Type::bytes())?;

    types::arguments(
        &observation.requested_arguments,
        &[source.clone(), Type::bytes()],
    )?;
    let selected = if *source == Type::Str {
        Vec::new()
    } else {
        vec![Type::u8()]
    };
    types::arguments(&observation.selected_arguments, &selected)?;

    let reference = source.clone().shared();
    observation
        .requested_signature
        .verify(&reference, &Type::bytes().shared())?;
    observation
        .signature
        .verify(&reference, &Type::bytes().shared())?;
    types::arguments(&observation.receiver_type, &[reference])?;
    types::arguments(&observation.result_type, &[Type::bytes().shared()])
}

fn adjustments(observation: &Adjustments, receiver: &HirNode, source: &Type) -> Result<(), String> {
    if observation.hir != receiver.hir || observation.span != receiver.span {
        return Err("AsRef receiver adjustment site disagrees".into());
    }
    let [dereference, borrow] = observation.steps.as_slice() else {
        return Err("AsRef requires exactly builtin dereference then shared borrow".into());
    };
    if dereference.kind != "builtin_deref" || borrow.kind != "shared_borrow" {
        return Err("AsRef receiver adjustment order or kind disagrees".into());
    }

    types::arguments(&observation.source_type, &[source.clone().shared()])?;
    types::arguments(&dereference.target, std::slice::from_ref(source))?;
    types::arguments(&borrow.target, &[source.clone().shared()])?;
    types::arguments(&observation.adjusted_type, &[source.clone().shared()])
}
