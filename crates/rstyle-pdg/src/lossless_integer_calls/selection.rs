//! Raw selected instances supply structural substitutions, never display authority.
use super::*;
use crate::data_library::Registry;
use serde_json::Value;

pub(super) fn verify(
    node: &HirNode,
    kind: Kind,
    facts: &dyn EvidenceFacts,
    registry: &Registry,
) -> Result<(), String> {
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Lossless conversion lacks original call instance inventory")?;
    let call = site(inventory, node)?;
    relationship(call, kind, registry)
}

pub(super) fn site<'a>(inventory: &'a Value, node: &HirNode) -> Result<&'a Value, String> {
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Lossless conversion requires complete original call inventory".into());
    }

    let calls = inventory["calls"]
        .as_array()
        .ok_or("Missing lossless call roster")?;
    let mut matching = calls.iter().filter(|call| {
        serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            == Some(&node.hir)
            || serde_json::from_value::<crate::ByteSpan>(call["span"].clone()).ok()
                == Some(node.span)
    });
    let call = matching.next().ok_or("Missing exact lossless call site")?;
    if matching.next().is_some()
        || serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            != Some(&node.hir)
        || serde_json::from_value::<crate::ByteSpan>(call["span"].clone()).ok() != Some(node.span)
        || call["status"] != "resolved"
        || call["expanded"] != false
        || call
            .get("compiled_dependency")
            .is_some_and(|value| !value.is_null())
    {
        return Err("Lossless call is incomplete, ambiguous or not original".into());
    }
    Ok(call)
}

pub(super) fn relationship(call: &Value, kind: Kind, registry: &Registry) -> Result<(), String> {
    definition(&call["requested"], kind.path(), registry)?;
    definition(
        &call["selected"],
        if kind == Kind::Convert {
            IMPLEMENTATION
        } else {
            EXPECT
        },
        registry,
    )?;

    let requested = arguments(&call["requested_arguments"])?;
    let selected = arguments(&call["selected_arguments"])?;
    if kind == Kind::Convert {
        if requested.len() != 2 || !selected.is_empty() {
            return Err("Lossless TryFrom substitutions disagree".into());
        }
        builtin(&requested[0], "u64")?;
        builtin(&requested[1], "usize")?;
    } else {
        if requested.len() != 2 || selected != requested || call["requested"] != call["selected"] {
            return Err("Lossless expect selected substitutions disagree".into());
        }
        builtin(&requested[0], "u64")?;
        error(&requested[1], registry)?;
    }
    signature(&call["signature"], kind)
}

fn arguments(value: &Value) -> Result<&[Value], String> {
    let arguments = value
        .as_array()
        .ok_or("Missing structural lossless call substitutions")?;
    if arguments.iter().any(|argument| {
        argument["kind"] != "type"
            || argument["display_only"] != true
            || argument["identity_status"] != "known"
            || argument["value"].as_str().is_none_or(str::is_empty)
    }) {
        return Err("Lossless call argument lacks known structural type identity".into());
    }
    Ok(arguments)
}

fn builtin(argument: &Value, expected: &str) -> Result<(), String> {
    if argument["identity"] != serde_json::json!({"kind":"builtin", "name":expected}) {
        return Err("Lossless call builtin substitution differs".into());
    }
    Ok(())
}

fn error(argument: &Value, registry: &Registry) -> Result<(), String> {
    let identity = &argument["identity"];
    if identity["kind"] != "adt" || identity["arguments"] != serde_json::json!([]) {
        return Err("Lossless expect error is not exact concrete TryFromIntError".into());
    }
    definition(&identity["definition"], ERROR, registry)
}

fn definition(value: &Value, expected: &str, registry: &Registry) -> Result<(), String> {
    let stable = registry
        .definitions
        .get(authority::STABLE_CORE)
        .filter(|identity| !identity.is_empty())
        .ok_or("Missing pinned core stable crate identity")?;
    let identity = registry
        .definitions
        .get(expected)
        .filter(|identity| !identity.is_empty())
        .ok_or("Missing pinned lossless core definition")?;
    if value["crate"] != "core"
        || value["local"] != false
        || value["stable_crate_id"].as_str() != Some(stable.as_str())
        || value["definition_identity"].as_str() != Some(identity.as_str())
        || !value["path"]
            .as_str()
            .is_some_and(|value| authority::path(value, expected))
    {
        return Err("Lossless selected instance differs from exact pinned core definition".into());
    }
    Ok(())
}

fn signature(value: &Value, kind: Kind) -> Result<(), String> {
    let result = "core::result::Result<u64, core::num::TryFromIntError>";
    let (inputs, output) = match kind {
        Kind::Convert => (serde_json::json!(["usize"]), result),
        Kind::Expect => (serde_json::json!([result, "&str"]), "u64"),
    };

    // Display strings only cross-check the independently proved substitutions.
    let normalize = |value: &Value| -> Value {
        match value {
            Value::String(text) => Value::String(text.replace("std::", "core::")),
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|item| match item {
                        Value::String(text) => Value::String(text.replace("std::", "core::")),
                        _ => item.clone(),
                    })
                    .collect(),
            ),
            _ => value.clone(),
        }
    };
    if value["display_only"] != true
        || normalize(&value["inputs"]) != inputs
        || normalize(&value["output"]) != output
    {
        return Err("Lossless raw signature disagrees with structural substitutions".into());
    }
    Ok(())
}
