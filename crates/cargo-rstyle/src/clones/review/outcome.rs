//! Shared presentation of independently checked exact review lookups.
use super::{local, source_rules};
use serde::Serialize;
use std::fmt::Debug;

#[derive(Serialize)]
pub(super) struct Outcome {
    pub state: &'static str,
    pub reason: Option<String>,
}

impl Outcome {
    pub fn checked(result: Result<Self, String>) -> Self {
        result.unwrap_or_else(|reason| Self {
            state: "unavailable",
            reason: Some(reason),
        })
    }

    fn exact(reason: &str) -> Self {
        Self {
            state: "hidden_exact",
            reason: Some(reason.into()),
        }
    }

    fn new() -> Self {
        Self {
            state: "visible_new",
            reason: None,
        }
    }

    fn stale<'a>(entries: impl Iterator<Item = (impl Debug, &'a str)>) -> Self {
        let reasons = entries
            .map(|(reason, choice)| format!("{reason:?}: {choice}"))
            .collect::<Vec<_>>()
            .join("; ");
        Self {
            state: "stale",
            reason: Some(reasons),
        }
    }
}

impl From<local::Lookup<'_>> for Outcome {
    fn from(lookup: local::Lookup<'_>) -> Self {
        match lookup {
            local::Lookup::HiddenExact(entry) => Self::exact(&entry.reason),
            local::Lookup::VisibleNew => Self::new(),
            local::Lookup::Stale(entries) => Self::stale(
                entries
                    .iter()
                    .map(|entry| (entry.reason, entry.entry.reason.as_str())),
            ),
        }
    }
}

impl From<source_rules::Lookup<'_>> for Outcome {
    fn from(lookup: source_rules::Lookup<'_>) -> Self {
        match lookup {
            source_rules::Lookup::HiddenExact(entry) => Self::exact(&entry.reason),
            source_rules::Lookup::VisibleNew => Self::new(),
            source_rules::Lookup::Stale(entries) => Self::stale(
                entries
                    .iter()
                    .map(|entry| (entry.reason, entry.entry.reason.as_str())),
            ),
        }
    }
}
