//! Pure exact review lifecycle. Loading bytes never grants current pair validity.
//! No hashes: full versioned member values are the only equality authority.
pub(super) mod baseline_common;
mod baseline_io;
mod comparison;
pub(super) mod compiler_baseline;
pub(super) mod compiler_diagnostics;
pub(super) mod compiler_owners;
pub(super) mod compiler_pattern_baseline;
pub(super) mod compiler_patterns;
mod exact_rules;
pub(super) mod local;
mod local_adapter;
mod model;
mod numeric_workflow;
mod outcome;
mod owner_observations;
mod owner_population;
mod positive_workflow;
pub(super) mod prune;
mod publication;
pub(super) mod status;
mod summary;
pub(super) mod workflow;
pub(crate) use model::*;

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewError {
    InvalidSchema,
    InvalidReason,
    InvalidVersion,
    InvalidContext,
    InvalidLocator,
    NoncanonicalMembers,
    DuplicateReview,
    ExpressionBudget,
}

impl Default for ReviewStore {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            compiler_pattern_entries: vec![],
            compiler_diagnostic_entries: vec![],
        }
    }
}

impl ReviewStore {
    pub(super) fn retained_entries(&self) -> usize {
        self.compiler_pattern_entries.len() + self.compiler_diagnostic_entries.len()
    }

    pub(crate) fn from_json(bytes: &str) -> Result<Self, String> {
        baseline_io::parse(bytes.as_bytes())
    }

    pub(crate) fn to_json(&self) -> Result<String, String> {
        String::from_utf8(baseline_io::encode(self)?).map_err(|error| error.to_string())
    }

    fn validate(&self) -> Result<(), ReviewError> {
        if !matches!(self.schema_version, 1..=10)
            || (self.schema_version < 8 && !self.compiler_pattern_entries.is_empty())
            || (self.schema_version < 9 && !self.compiler_diagnostic_entries.is_empty())
        {
            return Err(ReviewError::InvalidSchema);
        }
        compiler_patterns::validate_entries(&self.compiler_pattern_entries)?;
        compiler_diagnostics::validate_entries(&self.compiler_diagnostic_entries)?;
        Ok(())
    }
}

impl baseline_io::Artifact for ReviewStore {
    const LABEL: &'static str = "Review store";

    fn validate(&self) -> Result<(), String> {
        ReviewStore::validate(self).map_err(|error| format!("{error:?}"))
    }
}

fn relative_source(source: &str) -> bool {
    !source.is_empty()
        && !source.starts_with('/')
        && !source.contains('\\')
        && !source.contains(':')
        && source
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
}
