//! Explicit run configuration: only the profile's allowed overrides.
//!
//! Thresholds are finite decimal values in `[0, 1]`, parsed as exact
//! rationals from their lexemes. Unknown keys or variants are errors, and
//! the effective configuration is recorded in every report.

use crate::frac::{ArithError, Frac};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scoring {
    SmallerGraphOverlap,
    CrossProductCosine,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CcgraphConfig {
    pub minimum_size_ratio: Frac,
    pub numerical_threshold: Frac,
    pub string_threshold: Frac,
    pub final_threshold: Frac,
    pub scoring: Scoring,
}

impl Default for CcgraphConfig {
    fn default() -> Self {
        Self {
            minimum_size_ratio: decimal("0.8"),
            numerical_threshold: decimal("0.9"),
            string_threshold: decimal("0.5"),
            final_threshold: decimal("0.9"),
            scoring: Scoring::SmallerGraphOverlap,
        }
    }
}

fn decimal(lexeme: &str) -> Frac {
    Frac::parse_decimal(lexeme).expect("profile defaults are valid decimals")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    pub message: String,
}

fn config_error(message: String) -> ConfigError {
    ConfigError { message }
}

/// Parse a JSON object with only the allowed override keys.
pub fn parse_config(value: &serde_json::Value) -> Result<CcgraphConfig, ConfigError> {
    let object = value
        .as_object()
        .ok_or_else(|| config_error("configuration must be an object".to_string()))?;
    reject_unknown_keys(object)?;

    let fallback = CcgraphConfig::default();
    Ok(CcgraphConfig {
        minimum_size_ratio: read_threshold(
            object,
            "minimum_size_ratio",
            fallback.minimum_size_ratio,
        )?,
        numerical_threshold: read_threshold(
            object,
            "numerical_threshold",
            fallback.numerical_threshold,
        )?,
        string_threshold: read_threshold(object, "string_threshold", fallback.string_threshold)?,
        final_threshold: read_threshold(object, "final_threshold", fallback.final_threshold)?,
        scoring: read_scoring(object, fallback.scoring)?,
    })
}

fn reject_unknown_keys(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), ConfigError> {
    object
        .keys()
        .find(|key| !is_allowed(key))
        .map(|key| config_error(format!("unknown configuration key {key:?}")))
        .map_or(Ok(()), Err)
}

fn is_allowed(key: &str) -> bool {
    matches!(
        key,
        "minimum_size_ratio"
            | "numerical_threshold"
            | "string_threshold"
            | "final_threshold"
            | "scoring"
    )
}

fn read_threshold(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    current: Frac,
) -> Result<Frac, ConfigError> {
    match object.get(key) {
        None => Ok(current),
        Some(value) => {
            parse_threshold(value).map_err(|e| config_error(format!("{key}: {}", e.message)))
        }
    }
}

fn parse_threshold(value: &serde_json::Value) -> Result<Frac, ConfigError> {
    let lexeme = threshold_lexeme(value)?;
    let threshold = Frac::parse_decimal(&lexeme).map_err(|e| config_error(e.what.to_string()))?;
    check_range(threshold)?;
    Ok(threshold)
}

fn threshold_lexeme(value: &serde_json::Value) -> Result<String, ConfigError> {
    match value {
        serde_json::Value::String(text) => Ok(text.clone()),
        // Preserve the exact JSON number lexeme (e.g. 0.50000000000000001)
        // instead of reconstructing it after an f64 round trip.
        serde_json::Value::Number(number) => Ok(number.as_str().to_string()),
        _ => Err(config_error(
            "threshold must be a decimal string or number".to_string(),
        )),
    }
}

fn check_range(threshold: Frac) -> Result<(), ConfigError> {
    let in_range = threshold
        .ge(decimal("0"))
        .map_err(|e| config_error(e.what.to_string()))?
        && decimal("1")
            .ge(threshold)
            .map_err(|e| config_error(e.what.to_string()))?;
    if in_range {
        Ok(())
    } else {
        Err(config_error("threshold must lie in [0,1]".to_string()))
    }
}

fn read_scoring(
    object: &serde_json::Map<String, serde_json::Value>,
    current: Scoring,
) -> Result<Scoring, ConfigError> {
    match object.get("scoring") {
        None => Ok(current),
        Some(serde_json::Value::String(name)) => match name.as_str() {
            "smaller_graph_overlap" => Ok(Scoring::SmallerGraphOverlap),
            "cross_product_cosine" => Ok(Scoring::CrossProductCosine),
            _ => Err(config_error(format!("unknown scoring variant {name:?}"))),
        },
        Some(_) => Err(config_error("scoring must be a string".to_string())),
    }
}

impl From<ArithError> for ConfigError {
    fn from(_: ArithError) -> Self {
        config_error("rational arithmetic failed".to_string())
    }
}
