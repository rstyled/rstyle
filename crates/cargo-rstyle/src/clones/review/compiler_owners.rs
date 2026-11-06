//! Reuse captured compiler declarations for advisory review identities.
use crate::clones::build::{ResolvedTy, StringFact};
use crate::clones::{identity::UnitId, report as compiler_report};
use rstyle_core::{
    facts::{FnFacts, SigFacts},
    locator,
};
use serde::{Deserialize, Serialize, de::Error};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones::review) struct Signature {
    pub parameters: Vec<ResolvedTy>,
    pub result: ResolvedTy,
    #[serde(deserialize_with = "read_method")]
    pub method: Option<SigFacts>,
    pub abi: StringFact,
    pub generics: Vec<String>,
    pub has_self: bool,
    pub is_variadic: bool,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_const: bool,
}

pub(in crate::clones) fn originals<'a>(
    ctx: &'a compiler_report::Context<'_>,
    unit: UnitId,
) -> BTreeMap<&'a str, &'a crate::clones::Function> {
    ctx.handoff
        .assessed
        .iter()
        .filter(|record| record.key.unit() == unit)
        .filter_map(|record| record.facts.compiled_id().map(|id| (id, record)))
        .collect()
}

pub(super) struct Names {
    pub by_id: BTreeMap<String, Result<String, String>>,
    pub counts: BTreeMap<String, usize>,
}

pub(super) fn names<'a>(
    facts: impl IntoIterator<Item = &'a FnFacts>,
) -> BTreeMap<String, Result<String, String>> {
    collect_names(facts).by_id
}

/// Preserve the full locator census without retaining additional owner payloads.
pub(super) fn collect_names<'a>(facts: impl IntoIterator<Item = &'a FnFacts>) -> Names {
    let named: Vec<_> = facts
        .into_iter()
        .map(|facts| (&facts.id, locator::base(facts)))
        .collect();
    let mut counts = BTreeMap::new();
    for (_, name) in &named {
        *counts.entry(name.clone()).or_insert(0) += 1;
    }

    let by_id = named
        .into_iter()
        .map(|(id, name)| {
            let result = if counts[&name] != 1 || name.contains("{impl#") {
                Err("Compiler review locator is ambiguous or lacks a stable owner".into())
            } else {
                Ok(name)
            };
            (id.clone(), result)
        })
        .collect();
    Names { by_id, counts }
}

pub(super) fn signature(
    declared: &rstyle_core::cpg::records::DeclaredSignature,
    method: Option<rstyle_core::facts::SigFacts>,
) -> Signature {
    document_signature(declared, method)
}

/// What a document declares, read as its capture records it: a foreign ABI is the
/// only one the frontend tells apart.
fn document_signature(
    sig: &rstyle_core::cpg::records::DeclaredSignature,
    method: Option<rstyle_core::facts::SigFacts>,
) -> Signature {
    let resolved = |ty: &rstyle_core::cpg::records::DeclaredType| ResolvedTy {
        normalized: ty.normalized.clone(),
        observed: ty.observed.clone(),
        spelling: ty.spelling.clone(),
    };
    let has = |modifier: &str| sig.modifiers.iter().any(|name| name == modifier);
    let abi = if has("rust:foreign-abi") { "C" } else { "Rust" };
    Signature {
        parameters: sig.parameters.iter().map(resolved).collect(),
        result: resolved(&sig.result),
        method,
        abi: StringFact {
            status: crate::clones::build::FactStatus::Known,
            value: Some(abi.into()),
            reason: None,
        },
        generics: sig.generics.clone(),
        has_self: sig.has_self,
        is_variadic: has("variadic"),
        is_async: has("async"),
        is_unsafe: has("unsafe"),
        is_const: has("const"),
    }
}

fn read_method<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SigFacts>, D::Error> {
    let raw = serde_json::Value::deserialize(deserializer)?;
    rstyle_core::advisory::require_signature(&raw).map_err(D::Error::custom)?;
    Option::<SigFacts>::deserialize(raw).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests;
