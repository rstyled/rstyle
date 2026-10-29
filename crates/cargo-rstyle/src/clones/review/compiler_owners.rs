//! Reuse captured compiler declarations for advisory review identities.
use crate::clones::{identity::UnitId, report as compiler_report};
use rstyle_core::{
    facts::{FnFacts, SigFacts},
    locator,
};
use rstyle_pdg::{ResolvedTy, StringFact};
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

pub(super) fn originals<'a>(
    ctx: &'a compiler_report::Context<'_>,
    unit: UnitId,
) -> BTreeMap<&'a str, &'a crate::clones::Function> {
    ctx.handoff
        .assessed
        .iter()
        .filter(|record| record.key.unit() == unit)
        .filter_map(|record| {
            record
                .facts
                .view()
                .build
                .frontend_options
                .get(rstyle_core::advisory::KEY)?["facts"]["id"]
                .as_str()
                .map(|id| (id, record))
        })
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
    original: &rstyle_pdg::HirFnFacts,
    method: Option<rstyle_core::facts::SigFacts>,
) -> Signature {
    let sig = &original.signature;
    Signature {
        parameters: sig.params.iter().map(|param| param.ty.clone()).collect(),
        result: sig.return_ty.clone(),
        method,
        abi: sig.abi.clone(),
        generics: sig
            .generics
            .iter()
            .map(|param| param.name.clone())
            .collect(),
        has_self: sig.has_self,
        is_variadic: sig.is_variadic,
        is_async: sig.is_async,
        is_unsafe: sig.is_unsafe,
        is_const: sig.is_const,
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
