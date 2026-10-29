//! Direct collector events for expanded owners without authored source metrics.
use crate::facts::{CallEvent, EffectEvent, MutationEvent};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.compiler-generated-effects/1";

/// This record does not measure source size, purity or transitive effects.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub metrics: String,
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub mutations: Vec<MutationEvent>,
    pub calls: Vec<CallEvent>,
    pub intrinsic_effects: Vec<EffectEvent>,
}

impl Observation {
    pub fn read(raw: &serde_json::Value) -> Result<Self, String> {
        // The event structs also serve historical formats with defaulted fields.
        // An absent measurement must not become a default in this format.
        for (roster, fields) in [
            ("mutations", &["origin", "delegated"][..]),
            ("calls", &["dynamic"][..]),
        ] {
            for event in raw[roster]
                .as_array()
                .ok_or("Missing generated event roster")?
            {
                super::require_fields(event, fields)?;
            }
        }
        let observation = Self::deserialize(raw).map_err(|error| error.to_string())?;
        if observation.metrics != KEY
            || observation.name.is_empty()
            || observation.line == 0
            || observation.column == 0
        {
            return Err("Unknown generated effect metric or missing owner location".into());
        }
        Ok(observation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_rosters_and_defaulted_event_fields_are_not_measurements() {
        let raw = serde_json::json!({
            "metrics": KEY, "name": "generated", "line": 3, "column": 1,
            "mutations": [{"origin":"source", "root":"out", "field_path":"", "line":3,
                "escapes":true, "delegated":false}],
            "calls": [{"callee":"std::fs::read", "line":3, "dynamic":false}],
            "intrinsic_effects": [],
        });
        assert!(Observation::read(&raw).is_ok());
        for field in ["mutations", "calls", "intrinsic_effects"] {
            let mut missing = raw.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(Observation::read(&missing).is_err(), "{field}");
        }
        for (roster, field) in [
            ("mutations", "origin"),
            ("mutations", "delegated"),
            ("calls", "dynamic"),
        ] {
            let mut missing = raw.clone();
            missing[roster][0].as_object_mut().unwrap().remove(field);
            assert!(Observation::read(&missing).is_err(), "{roster}.{field}");
        }
        let mut unknown = raw;
        unknown["metrics"] = "future".into();
        assert!(Observation::read(&unknown).is_err());
    }
}
