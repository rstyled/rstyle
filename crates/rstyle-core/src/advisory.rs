//! Versioned compiler observations for advisory consumers, never clone authority.
use crate::facts::FnFacts;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.compiler-advisory/1";
pub const METRICS: &str = "rstyle.compiler-style-metrics/1";
pub const CAPTURE_ENV: &str = "RSTYLE_COMPILER_ADVISORY";

/// Absence of this record means not captured, rather than an empty body.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    Known {
        metrics: String,
        facts: Box<FnFacts>,
    },
    Unavailable {
        reason: String,
    },
}

impl Observation {
    /// Read this metric version without accepting historical serde zero defaults.
    pub fn read(raw: &serde_json::Value) -> Result<Self, String> {
        if raw["status"] == "known" {
            require_fields(
                &raw["facts"],
                &[
                    "id",
                    "implements",
                    "param_list",
                    "top_statements",
                    "edges",
                    "mutated_types",
                    "sig",
                    "pdg",
                ],
            )?;
            for (roster, fields) in [
                ("param_list", &["uses"][..]),
                ("mutations", &["delegated"][..]),
                ("calls", &["dynamic"][..]),
                ("edges", &["dynamic", "args"][..]),
            ] {
                for entry in raw["facts"][roster]
                    .as_array()
                    .ok_or("Missing compiler advisory roster")?
                {
                    require_fields(entry, fields)?;
                }
            }
        }
        let observation = Self::deserialize(raw).map_err(|error| error.to_string())?;
        match &observation {
            Self::Known { metrics, .. } if metrics != METRICS => {
                return Err("Unknown compiler advisory metric version".into());
            }
            Self::Unavailable { reason } if reason.is_empty() => {
                return Err("Missing compiler advisory unavailability reason".into());
            }
            _ => {}
        }
        Ok(observation)
    }
}

fn require_fields(raw: &serde_json::Value, required: &[&str]) -> Result<(), String> {
    let fields = raw
        .as_object()
        .ok_or("Compiler advisory facts are not an object")?;
    if required.iter().any(|key| !fields.contains_key(*key)) {
        return Err(
            "Compiler advisory observations omit required fields; legacy defaults are unavailable"
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
