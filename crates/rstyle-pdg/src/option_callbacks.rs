//! Immediate Option callback observations and source contracts.
//! These checks alone never authorize graph admission.

mod invocation;
mod predicate;
pub(crate) mod source;
pub(crate) use invocation::{Arguments, verify_arguments};
pub use invocation::{Invocation, verify_map_or};
pub use predicate::{Predicate, verify_predicate};

use crate::{HirRef, concrete_types::Observation};
use serde::{Deserialize, Serialize};

pub const ENVIRONMENT_KIND: &str = "compiler_closure_environment";

pub(crate) fn candidate(
    node: &crate::HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> bool {
    node.kind == "expr_method_call"
        && crate::extraction_profile::facts_support(facts, "rust.immediate-option-callbacks")
        && crate::data_library::callee(node).is_ok_and(|callee| {
            matches!(
                callee.path.as_str(),
                "std::option::Option::<T>::map_or" | "core::option::Option::<T>::map_or"
            )
        })
}

fn candidates(facts: &dyn crate::unit_store::EvidenceFacts) -> Vec<&crate::HirNode> {
    if !crate::extraction_profile::facts_support(facts, "rust.immediate-option-callbacks") {
        return Vec::new();
    }
    let mut methods = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut methods);
    methods
        .into_iter()
        .filter(|node| candidate(node, facts))
        .collect()
}

pub(crate) enum Kind {
    Closure,
    FunctionItem,
}

pub(crate) fn invocations(
    facts: &dyn crate::unit_store::EvidenceFacts,
    kind: Kind,
) -> Vec<&crate::HirNode> {
    let named = matches!(kind, Kind::FunctionItem);
    candidates(facts)
        .into_iter()
        .filter(|node| crate::named_callbacks::candidate(node, facts) == named)
        .collect()
}

pub(crate) fn verify_environment_event(
    source: &crate::HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let call = invocations(facts, Kind::Closure)
        .into_iter()
        .find(|call| {
            call.children
                .last()
                .is_some_and(|callback| callback.hir == source.hir)
        })
        .ok_or("Environment event is not an immediate Option callback")?;
    verify_map_or(call, facts)?;
    Ok(())
}

/// Stored on the closure node itself, without another global source registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub captures: Vec<Capture>,
    pub stored_type: Observation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub root: HirRef,
    pub mode: CaptureMode,
    /// Nonzero projections are outside the initial contract. Retaining their
    /// count prevents a projected capture from masquerading as a whole scalar.
    pub projections: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
    Shared,
    Unique,
    Mutable,
    Value,
    Use,
}
