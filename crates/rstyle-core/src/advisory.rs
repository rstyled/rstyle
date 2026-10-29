//! Versioned compiler observations for advisory consumers, never clone authority.
use crate::facts::FnFacts;
use serde::{Deserialize, Serialize};

pub mod patterns;

pub const KEY: &str = "rstyle.compiler-advisory/1";
pub const METRICS: &str = "rstyle.compiler-style-metrics/6";
pub const V5_METRICS: &str = "rstyle.compiler-style-metrics/5";
pub const V4_METRICS: &str = "rstyle.compiler-style-metrics/4";
pub const V3_METRICS: &str = "rstyle.compiler-style-metrics/3";
pub const V2_METRICS: &str = "rstyle.compiler-style-metrics/2";
pub const LEGACY_METRICS: &str = "rstyle.compiler-style-metrics/1";
pub const SOURCE_ROOT_ENV: &str = "RSTYLE_COMPILER_SOURCE_ROOT";
pub const CAPTURE_ENV: &str = "RSTYLE_COMPILER_ADVISORY";

/// Historical observations keep their measured semantics on replay.
pub fn metric_version(value: &str) -> Option<&'static str> {
    match value {
        METRICS => Some(METRICS),
        LEGACY_METRICS => Some(LEGACY_METRICS),
        V2_METRICS => Some(V2_METRICS),
        V3_METRICS => Some(V3_METRICS),
        V4_METRICS => Some(V4_METRICS),
        V5_METRICS => Some(V5_METRICS),
        _ => None,
    }
}

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
                    if roster == "mutations"
                        && matches!(
                            raw["metrics"].as_str(),
                            Some(V2_METRICS | V3_METRICS | V4_METRICS | V5_METRICS | METRICS)
                        )
                    {
                        require_fields(entry, &["origin"])?;
                    }
                }
            }
            require_signature(&raw["facts"]["sig"])?;
            if raw["metrics"] == METRICS {
                require_fields(&raw["facts"], &["complexity"])?;
            } else if !raw["facts"]["complexity"].is_null() {
                return Err(
                    "Historical compiler metric cannot claim complexity measurements".into(),
                );
            }
        }
        let observation = Self::deserialize(raw).map_err(|error| error.to_string())?;
        observation.validate()?;
        Ok(observation)
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Known { metrics, .. } if metric_version(metrics).is_none() => {
                return Err("Unknown compiler advisory metric version".into());
            }
            Self::Known { metrics, facts }
                if metrics == LEGACY_METRICS
                    && facts
                        .mutations
                        .iter()
                        .any(|event| event.origin != crate::facts::MutationOrigin::Source) =>
            {
                return Err(
                    "Legacy compiler metric cannot claim desugared mutation origins".into(),
                );
            }
            Self::Unavailable { reason } if reason.is_empty() => {
                return Err("Missing compiler advisory unavailability reason".into());
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn require_signature(raw: &serde_json::Value) -> Result<(), String> {
    if !raw.is_null() {
        require_fields(
            raw,
            &[
                "self_ty",
                "self_args",
                "params",
                "is_async",
                "is_unsafe",
                "is_const",
                "generics",
            ],
        )?;
    }
    Ok(())
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
