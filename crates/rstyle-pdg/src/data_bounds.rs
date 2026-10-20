//! Typed normal-completion requirements; these records do not prove totality.

use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Bounds {
    Index {
        rule: &'static str,
        base_operand: u64,
        index_operand: u64,
        array_length: Option<u64>,
        exceptional_flow: &'static str,
    },
    Subslice {
        rule: &'static str,
        base_operand: u64,
        start_operand: u64,
        end_operand: u64,
        exceptional_flow: &'static str,
    },
}

impl Bounds {
    pub fn index(array_length: Option<u64>) -> Self {
        Self::Index {
            rule: "index-less-than-length",
            base_operand: 0,
            index_operand: 1,
            array_length,
            exceptional_flow: "omitted",
        }
    }

    pub fn subslice() -> Self {
        Self::Subslice {
            rule: "start-less-or-equal-end-less-or-equal-length",
            base_operand: 0,
            start_operand: 1,
            end_operand: 2,
            exceptional_flow: "omitted",
        }
    }

    pub fn value(self) -> Result<serde_json::Value, String> {
        serde_json::to_value(self).map_err(|error| error.to_string())
    }
}
