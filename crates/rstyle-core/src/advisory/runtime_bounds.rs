//! Opt-in raw semantic CFG observations; no analysis result is capture authority.
use crate::runtime_bounds::Program;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.compiler-runtime-bounds/2";
pub const CAPTURE_ENV: &str = "RSTYLE_COMPILER_RUNTIME_BOUNDS";
pub const MAX_NODES: usize = 8192;
pub const MAX_VARIABLES: usize = 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: String,
    pub owner: String,
    pub input: Input,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Captured { program: Program },
    BudgetExhausted { reason: String },
    Unavailable { reason: String },
}

impl Observation {
    pub fn read(raw: &serde_json::Value) -> Result<Self, String> {
        check_size(raw)?;
        if raw["input"]["program"]["nodes"]
            .as_array()
            .is_some_and(|v| v.len() > MAX_NODES)
            || raw["input"]["program"]["inputs"]
                .as_array()
                .is_some_and(|v| v.len() > MAX_VARIABLES)
            || raw["input"]["program"]["loop_headers"]
                .as_array()
                .is_some_and(|v| v.len() > MAX_NODES)
            || raw["input"]["program"]["variables"]
                .as_array()
                .is_some_and(|v| v.len() > MAX_VARIABLES)
        {
            return Err("Runtime-bound observation exceeds raw input limits".into());
        }

        let observation = Self::deserialize(raw).map_err(|error| error.to_string())?;
        if observation.schema != KEY || observation.owner.is_empty() {
            return Err("Unknown runtime-bound observation schema or missing owner".into());
        }
        Ok(observation)
    }
}

// Bound reader traversal and string bytes before allocating typed CFG copies.
fn check_size(raw: &serde_json::Value) -> Result<(), String> {
    let mut pending = vec![raw];
    let mut values = 0usize;
    let mut bytes = 0usize;

    while let Some(value) = pending.pop() {
        values += 1;
        match value {
            serde_json::Value::String(text) => bytes = bytes.saturating_add(text.len()),
            serde_json::Value::Array(items) => {
                if items.len().saturating_add(pending.len()) > MAX_NODES * 32 {
                    return Err("Runtime-bound observation exceeds raw input limits".into());
                }
                pending.extend(items);
            }
            serde_json::Value::Object(fields) => {
                if fields.len().saturating_add(pending.len()) > MAX_NODES * 32 {
                    return Err("Runtime-bound observation exceeds raw input limits".into());
                }
                bytes = bytes.saturating_add(fields.keys().map(String::len).sum::<usize>());
                pending.extend(fields.values());
            }
            _ => {}
        }
        if values > MAX_NODES * 32 || bytes > 8 * 1024 * 1024 || pending.len() > MAX_NODES * 32 {
            return Err("Runtime-bound observation exceeds raw input limits".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn unavailable_and_budget_are_recorded_observations_not_missing_inputs() {
        for status in ["unavailable", "budget_exhausted"] {
            let raw =
                json!({"schema":KEY,"owner":"owner","input":{"status":status,"reason":"recorded"}});
            assert!(Observation::read(&raw).is_ok());
        }
        assert!(Observation::read(&json!({"schema":KEY,"owner":"owner"})).is_err());
        assert!(Observation::read(&json!({"schema":"old","owner":"owner","input":{"status":"unavailable","reason":"x"}})).is_err());
    }
    #[test]
    fn oversized_raw_cfg_is_rejected_before_typed_copy() {
        let raw = json!({"schema":KEY,"owner":"owner","input":{"status":"captured","program":{"nodes":vec![json!(null);MAX_NODES+1]}}});
        assert!(
            Observation::read(&raw)
                .unwrap_err()
                .contains("raw input limits")
        );
    }
}
