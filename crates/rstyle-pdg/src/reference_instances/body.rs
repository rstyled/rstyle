//! Complete bounded constructor source/value-flow checks; no admission authority.
mod library;
mod model;
mod preserved;
mod projection;
mod source;
#[cfg(test)]
mod tests;

use crate::{HirFnFacts, HirRef};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, String>;
type Site = (u32, u32, u32);

fn site(hir: &HirRef) -> Site {
    (hir.krate, hir.owner, hir.local)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Formal,
    Bytes,
    Input,
    Span,
    Anchored,
    Usize,
    Bool,
}

/// This observation proof preserves the canonical original owner. Successful
/// checking does not create a concrete frontend owner or returned-origin summary.
pub(super) fn verify(canonical: &HirFnFacts, observation: &Value, haystack: &str) -> Result<()> {
    let record = model::Observation::deserialize(observation).map_err(|error| error.to_string())?;
    preserved::verify(canonical, &record, observation, haystack)?;

    let checked = source::verify(canonical)?;
    projection::verify(canonical, &record, &checked.types, haystack)?;
    projection::paths(canonical, &checked.nominal_paths)?;
    super::operations::verify(canonical, &record.conversions, haystack)?;
    library::verify(canonical, checked.length)
}

struct CheckedSource<'a> {
    types: BTreeMap<Site, Role>,
    length: &'a crate::HirNode,
    nominal_paths: Vec<(Role, syn::TypePath)>,
}
