//! Bounded compiler observations of direct mutable-u32 formals.
//! This supplies type evidence only, never access, alias or graph permission.
use crate::{HirFnFacts, HirRef};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "consumer_formal_types";
pub const IDENTITY: &str = "rstyle.consumer-formal-types/1";
pub const MAX_FORMALS: usize = 64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observations {
    pub identity: String,
    /// Only compiler Ref(Mut, Uint(U32)) entries; absence is unavailable.
    pub mutable_u32: Vec<Formal>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Formal {
    pub parameter: u32,
    pub binding: HirRef,
}

/// Requires original source/signature ownership checks by the consuming analysis.
/// Rendered type names and source spelling are deliberately never consulted.
pub fn mutable_u32(facts: &HirFnFacts, parameter: u32, binding: &HirRef) -> Result<bool, String> {
    let Some(value) = facts.build.frontend_options.get(KEY) else {
        return Ok(false);
    };
    if facts.signature.params.len() > MAX_FORMALS
        || value
            .get("mutable_u32")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|entries| entries.len() > MAX_FORMALS)
    {
        return Ok(false);
    }
    let observations = Observations::deserialize(value)
        .map_err(|error| format!("Invalid consumer formal types: {error}"))?;
    if observations.identity != IDENTITY {
        return Err("Consumer formal types have another schema identity".into());
    }

    let mut previous = None;
    let mut found = false;
    for formal in &observations.mutable_u32 {
        if previous.is_some_and(|previous| previous >= formal.parameter) {
            return Err("Consumer formal types repeat or reorder a formal".into());
        }
        let captured = facts
            .signature
            .params
            .get(formal.parameter as usize)
            .ok_or("Consumer formal type has no original parameter")?;
        if captured.position != formal.parameter || captured.pat.hir != formal.binding {
            return Err("Consumer formal type differs from original parameter identity".into());
        }
        found |= formal.parameter == parameter && &formal.binding == binding;
        previous = Some(formal.parameter);
    }
    Ok(found)
}
