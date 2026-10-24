//! Exact concrete owning conversion selection; never source, cleanup or effects authority.
mod authority;
#[cfg(test)]
mod tests;
mod verify;

use crate::tryfrom_calls::model::{self, Observation};
use crate::{HirFnFacts, concrete_types::TypeTree, nominal_boundary::census::expansions::FileSpan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Wrap,
    Unwrap,
}

/// Supplied by independently verified complete source/semantic ownership.
/// This observational leaf cannot turn a constructed contract into admission.
pub(crate) struct Contract<'a> {
    pub direction: Direction,
    pub input: &'a TypeTree,
    pub output: &'a TypeTree,
    pub implementation_source: &'a FileSpan,
    pub method_source: &'a FileSpan,
    pub error_source: Option<&'a FileSpan>,
}

pub(crate) fn verify(facts: &HirFnFacts, expected: Contract<'_>) -> Result<(), String> {
    let raw = facts
        .build
        .frontend_options
        .get(crate::tryfrom_calls::KEY)
        .ok_or("Missing original concrete conversion member observation")?;

    #[derive(serde::Deserialize)]
    #[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
    enum Outcome {
        Observed { observation: Box<Observation> },
        Unsupported { reason: String },
    }
    let observation =
        match serde_json::from_value(raw.clone()).map_err(|error| error.to_string())? {
            Outcome::Observed { observation } => *observation,
            Outcome::Unsupported { reason } => return Err(reason),
        };

    verify::captured(facts, &expected)?;
    let owner = verify::Owner::from_facts(facts)?;
    let pins = authority::reviewed(facts)?;
    verify::receipt(&observation, &expected, &owner, &pins)
}
