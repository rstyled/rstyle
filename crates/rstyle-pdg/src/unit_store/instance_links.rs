//! Selected scalar instances remain separate owners with typed lookup identities.
use super::*;
use crate::scalar_instances::{Projection, ScalarArgument};
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
mod tests;

const KEY: &str = "rstyle.rust.scalar-instance-projections";
const IDENTITY: &str = "rstyle.rust.scalar-instance-projections/1";

pub(super) type Key = (usize, FunctionId, ScalarArgument);

pub(super) fn argument(
    facts: &HirFnFacts,
    node: &HirNode,
) -> Result<Option<ScalarArgument>, String> {
    if !node.res.as_ref().is_some_and(|resolution| {
        resolution.kind == "resolved_scalar_instance_callee" && resolution.target.is_none()
    }) {
        return Ok(None);
    }
    crate::helper_calls::instances::call_argument(node, facts).map(Some)
}

pub(super) fn available(
    owner: &UnitStore,
    definition: FunctionId,
    argument: ScalarArgument,
) -> Result<bool, String> {
    Ok(selected(owner, definition, argument)?["status"] == "projected")
}

pub(super) fn capture(
    owner: Arc<UnitStore>,
    definition: FunctionId,
    argument: ScalarArgument,
) -> Result<Option<Arc<UnitStore>>, String> {
    let entry = selected(&owner, definition, argument)?;
    if entry["status"] != "projected" {
        return Ok(None);
    }

    let mut body = entry
        .as_object()
        .ok_or("Invalid scalar projection entry")?
        .clone();
    body.remove("request");
    body.remove("status");
    let projection: Projection =
        serde_json::from_value(Value::Object(body)).map_err(|e| e.to_string())?;
    crate::verification_policy::strict(|| {
        UnitStore::capture_scalar_instance(owner, definition.0 as usize, &projection)
            .map(Arc::new)
            .map(Some)
    })
}

fn selected(
    owner: &UnitStore,
    definition: FunctionId,
    argument: ScalarArgument,
) -> Result<&Value, String> {
    if owner.instance.is_some() {
        return Err("Scalar linking requires the immutable original definition owner".into());
    }
    let facts = owner
        .functions
        .get(definition.0 as usize)
        .ok_or("Missing scalar original definition")?;
    let entries = inventory(facts)?;
    Ok(entries[usize::from(argument == ScalarArgument::U32)])
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    function: String,
    arguments: Vec<String>,
}

fn inventory(facts: &HirFnFacts) -> Result<[&Value; 2], String> {
    let inventory = facts
        .build
        .frontend_options
        .get(KEY)
        .ok_or("Missing original scalar projection inventory")?;
    if inventory["identity"] != IDENTITY || inventory["complete"] != true {
        return Err("Incomplete original scalar projection inventory".into());
    }
    let entries = inventory["requests"]
        .as_array()
        .ok_or("Missing scalar projection request roster")?;
    if entries.len() != 2 {
        return Err("Scalar projection inventory requires both exact requests".into());
    }

    let mut slots = [None, None];
    for entry in entries {
        let request: Request =
            serde_json::from_value(entry["request"].clone()).map_err(|e| e.to_string())?;
        if request.function != facts.name {
            return Err("Scalar projection request names another original function".into());
        }
        let slot = match request.arguments.as_slice() {
            [scalar] if scalar == "i32" => 0,
            [scalar] if scalar == "u32" => 1,
            _ => return Err("Scalar projection request lacks one exact i32/u32 argument".into()),
        };
        status(entry)?;
        if slots[slot].replace(entry).is_some() {
            return Err("Duplicate original scalar projection request".into());
        }
    }
    let [Some(signed), Some(unsigned)] = slots else {
        return Err("Missing original scalar projection request".into());
    };
    Ok([signed, unsigned])
}

fn status(entry: &Value) -> Result<(), String> {
    match entry["status"].as_str() {
        Some("projected") if entry.get("reason").is_none() => Ok(()),
        Some("unsupported" | "unresolved" | "error" | "unavailable") => {
            let object = entry
                .as_object()
                .ok_or("Invalid unavailable scalar request")?;
            if object.len() != 3 || entry["reason"].as_str().is_none_or(str::is_empty) {
                return Err("Unavailable scalar request requires its exact recorded reason".into());
            }
            Ok(())
        }
        _ => Err("Missing or unknown original scalar projection status".into()),
    }
}
