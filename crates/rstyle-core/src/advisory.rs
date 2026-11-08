//! Versioned compiler observations for advisory consumers, never clone authority.
use crate::facts::FnFacts;
use serde::{Deserialize, Serialize};

pub mod generated_effects;
pub mod patterns;
pub mod runtime_bounds;

pub const KEY: &str = "rstyle.compiler-advisory/1";
pub const METRICS: &str = "rstyle.compiler-style-metrics/7";
pub const V6_METRICS: &str = "rstyle.compiler-style-metrics/6";
pub const V5_METRICS: &str = "rstyle.compiler-style-metrics/5";
pub const V4_METRICS: &str = "rstyle.compiler-style-metrics/4";
pub const V3_METRICS: &str = "rstyle.compiler-style-metrics/3";
pub const V2_METRICS: &str = "rstyle.compiler-style-metrics/2";
pub const LEGACY_METRICS: &str = "rstyle.compiler-style-metrics/1";
pub const SOURCE_ROOT_ENV: &str = "RSTYLE_COMPILER_SOURCE_ROOT";

/// Historical observations keep their measured semantics on replay.
pub fn metric_version(value: &str) -> Option<&'static str> {
    match value {
        METRICS => Some(METRICS),
        LEGACY_METRICS => Some(LEGACY_METRICS),
        V2_METRICS => Some(V2_METRICS),
        V3_METRICS => Some(V3_METRICS),
        V4_METRICS => Some(V4_METRICS),
        V5_METRICS => Some(V5_METRICS),
        V6_METRICS => Some(V6_METRICS),
        _ => None,
    }
}

/// Absence of this record means not captured, rather than an empty body.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// Versions that retained measured complexity independently of source availability.
pub fn has_complexity(metrics: &str) -> bool {
    matches!(metrics, METRICS | V6_METRICS)
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
