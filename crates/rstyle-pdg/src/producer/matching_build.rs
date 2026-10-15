//! Minimal versioned compatibility settings; full evidence stays in the frontend.
use crate::{BoolFact, StringFact, frontend::BuildFacts};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const KEY: &str = "matching_build";

#[derive(Serialize, Deserialize)]
enum Version {
    #[serde(rename = "rstyle.matching-build/1")]
    V1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: Version,
    overflow_checks: BoolFact,
    debug_assertions: BoolFact,
    panic_strategy: StringFact,
}

pub(super) fn capture(build: &BuildFacts) -> Value {
    serde_json::to_value(Record {
        schema: Version::V1,
        overflow_checks: build.overflow_checks.clone(),
        debug_assertions: build.debug_assertions.clone(),
        panic_strategy: build.panic_strategy.clone(),
    })
    .expect("matching settings serialize")
}

/// Decode the closed settings contract; returned values retain their exact facts.
pub fn settings(value: &Value) -> Result<BTreeMap<&str, &Value>, String> {
    let record = Record::deserialize(value).map_err(|error| error.to_string())?;
    if !record.overflow_checks.shape_ok()
        || !record.debug_assertions.shape_ok()
        || !record.panic_strategy.shape_ok()
    {
        return Err("Malformed compatible build settings".into());
    }

    Ok(["overflow_checks", "debug_assertions", "panic_strategy"]
        .into_iter()
        .map(|key| (key, &value[key]))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn value() -> Value {
        json!({"schema":"rstyle.matching-build/1",
            "overflow_checks":BoolFact::known(true),
            "debug_assertions":BoolFact::known(false),
            "panic_strategy":StringFact::known("unwind")})
    }

    #[test]
    fn compatibility_settings_preserve_exact_fact_values_and_order() {
        let value = value();
        let settings = settings(&value).unwrap();
        assert_eq!(
            serde_json::to_value(settings).unwrap(),
            json!({
            "overflow_checks":value["overflow_checks"],
            "debug_assertions":value["debug_assertions"],
            "panic_strategy":value["panic_strategy"]})
        );
        assert_eq!(value.as_object().unwrap().len(), 4);
    }

    #[test]
    fn missing_unknown_and_malformed_settings_cannot_pass_the_contract() {
        for key in [
            "schema",
            "overflow_checks",
            "debug_assertions",
            "panic_strategy",
        ] {
            let mut changed = value();
            changed.as_object_mut().unwrap().remove(key);
            assert!(settings(&changed).is_err());
        }
        let mut changed = value();
        changed["schema"] = json!("rstyle.matching-build/2");
        assert!(settings(&changed).is_err());
        changed = value();
        changed["captured_build"] = json!({});
        assert!(settings(&changed).is_err());
        changed = value();
        changed["overflow_checks"]["value"] = json!("true");
        assert!(settings(&changed).is_err());
    }
}
