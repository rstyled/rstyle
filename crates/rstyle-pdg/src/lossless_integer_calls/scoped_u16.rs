//! Callback-scoped blanket conversion authority, including every concrete obligation.
use super::*;
use crate::data_library::Registry;
use serde_json::{Value, json};

pub(crate) const CAPABILITY: &str = "rust.named-error-u16-usize-reader-flow";
pub(crate) const CONSTRUCT: &str = "data.integer.lossless-try-u16-usize";
const RECEIPT: &str = "rstyle.rust.usize-reader-conversion-closure/1";
const BASIS: &str = "independent concrete trait obligations corresponding to reviewed core convert/mod.rs blanket TryFrom/Into bodies; not nested source/HIR calls or an effect proof";
const CONVERT_SOURCE: &str =
    "sha256:ec1dfed5b8eef22496fc85eeb0b26a5600944fb8d2f6d6e9aaf81fb45af06de5";

type Pin = (&'static str, &'static str);
struct Stage {
    pins: [Pin; 5],
    arguments: [&'static str; 2],
    selected_arguments: &'static [&'static str],
    result: bool,
}

const TRY_FROM: Stage = Stage {
    pins: [
        (CONVERT, CONVERT),
        ("rust.concrete-tryfrom.trait", "core::convert::TryFrom"),
        ("rust.concrete-tryfrom.trait", "core::convert::TryFrom"),
        (
            "rust.u16-usize.tryfrom-method",
            "<T as core::convert::TryFrom<U>>::try_from",
        ),
        (
            "rust.u16-usize.tryfrom-impl",
            "<T as core::convert::TryFrom<U>>",
        ),
    ],
    arguments: ["usize", "u16"],
    selected_arguments: &["usize", "u16"],
    result: true,
};
const INTO: Stage = Stage {
    pins: [
        (
            "rust.product-conversion.into-method",
            "core::convert::Into::into",
        ),
        ("rust.product-conversion.into-trait", "core::convert::Into"),
        ("rust.product-conversion.into-trait", "core::convert::Into"),
        (
            "rust.product-conversion.into-blanket-method",
            "<T as core::convert::Into<U>>::into",
        ),
        (
            "rust.product-conversion.into-blanket-impl",
            "<T as core::convert::Into<U>>",
        ),
    ],
    arguments: ["u16", "usize"],
    selected_arguments: &["u16", "usize"],
    result: false,
};
const FROM: Stage = Stage {
    pins: [
        ("std::convert::From::from", "core::convert::From::from"),
        ("rust.product-conversion.from-trait", "core::convert::From"),
        ("rust.product-conversion.from-trait", "core::convert::From"),
        (
            "std::convert::num::<impl std::convert::From<u16> for usize>::from",
            "core::convert::num::<impl core::convert::From<u16> for usize>::from",
        ),
        (
            "rust.u16-usize.from-impl",
            "core::convert::num::<impl core::convert::From<u16> for usize>",
        ),
    ],
    arguments: ["usize", "u16"],
    selected_arguments: &[],
    result: false,
};

/// Only the complete reader family may call this; ordinary call resolution has no fallback.
pub(crate) fn verify(
    node: &HirNode,
    chain: &[&HirNode],
    facts: &dyn EvidenceFacts,
) -> Result<Summary, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err(
            "Scoped u16 conversion requires its registered complete reader capability".into(),
        );
    }
    let registry = reviewed(facts)?;
    interface(node, registry)?;
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Scoped u16 conversion lacks its original call inventory")?;
    let call = selection::scoped_site(inventory, node, chain)?;
    receipt(call, node, registry)?;
    original_source(node, facts)?;

    Ok(Summary {
        id: format!("rstyle.{CONSTRUCT}"),
        version: "1.0.0".into(),
        callee_path: node
            .dispatch
            .as_ref()
            .ok_or("Missing scoped u16 dispatch")?
            .op
            .clone(),
        callee_crate: "core".into(),
        argument_types: vec!["u16".into()],
        result_type: result_token()?,
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "reviewed-64-bit-u16-to-usize-always-ok-never-error".into(),
    })
}

fn reviewed(facts: &dyn EvidenceFacts) -> Result<&'static Registry, String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned scoped u16 conversion compiler/target")?;
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "scoped u16 conversion")?;
    authority::target(&facts.build.pointer_width, registry)?;

    if registry
        .result_propagation_source_digests
        .get("convert/mod.rs")
        .map(String::as_str)
        != Some(CONVERT_SOURCE)
    {
        return Err("Scoped u16 conversion lacks reviewed blanket conversion source".into());
    }

    super::u16_source::verify()?;
    Ok(registry)
}

fn result_token() -> Result<String, String> {
    crate::data_types::token(&crate::widening_readers::conversion_type(
        crate::native_reader_width::Width::U16,
    ))
}

fn interface(node: &HirNode, registry: &Registry) -> Result<(), String> {
    let [operand] = node.children.as_slice() else {
        return Err("Scoped u16 conversion requires its sole original operand".into());
    };
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing scoped u16 dispatch")?;
    let callee = crate::data_library::callee(node)?;
    let output = result_token()?;

    if node.kind != "expr_call"
        || node.expansion.is_some()
        || node.is_unsafe_block
        || dispatch.kind != "resolved_direct"
        || !authority::path(&dispatch.op, CONVERT)
        || dispatch.operand_tys != ["u16"]
        || crate::data_aggregates::token(operand)? != "u16"
        || dispatch.result_ty != output
        || crate::data_aggregates::token(node)? != output
        || callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry.definitions.get(CONVERT) != Some(&callee.definition_hash)
        || !authority::path(&callee.path, CONVERT)
    {
        return Err("Scoped u16 conversion changes its exact concrete interface".into());
    }
    if let Some(implementation) = &callee.implementation
        && (implementation.implementation.is_some()
            || !registry.exact_core_definition(implementation, "rust.u16-usize.tryfrom-method")
            || !same_path(&implementation.path, TRY_FROM.pins[3].1))
    {
        return Err("Scoped u16 callee changes its exact blanket implementation".into());
    }
    Ok(())
}

fn receipt(call: &Value, node: &HirNode, registry: &Registry) -> Result<(), String> {
    let receipt = call
        .get("usize_reader_conversion")
        .ok_or("Scoped u16 conversion lacks its concrete obligation closure")?;
    if receipt["identity"] != RECEIPT
        || receipt["selection_basis"] != BASIS
        || crate::source_evidence::call_inventory::decode::<crate::HirRef>(&receipt["hir"])?
            != node.hir
        || crate::source_evidence::call_inventory::decode::<crate::ByteSpan>(&receipt["span"])?
            != node.span
    {
        return Err("Scoped u16 conversion receipt changes its exact original source site".into());
    }
    let root = &receipt["try_from"];
    for field in [
        "requested",
        "requested_arguments",
        "selected",
        "selected_arguments",
        "signature",
        "status",
    ] {
        if call[field] != root[field] {
            return Err(
                "Scoped u16 conversion root differs from its original selected call".into(),
            );
        }
    }

    for (name, stage) in [("try_from", &TRY_FROM), ("into", &INTO), ("from", &FROM)] {
        selected(&receipt[name], stage, registry)?;
    }
    Ok(())
}

fn selected(value: &Value, stage: &Stage, registry: &Registry) -> Result<(), String> {
    if value["status"] != "resolved"
        || value.get("reason").is_some()
        || value["structural_signature"]["bound_variables"] != 0
    {
        return Err("Scoped u16 obligation lacks a complete resolved structural signature".into());
    }
    for (field, pin) in [
        "requested",
        "requested_trait",
        "selected_trait",
        "selected",
        "selected_impl",
    ]
    .into_iter()
    .zip(stage.pins)
    {
        selection::pinned_definition(&value[field], pin, registry)?;
    }

    arguments(&value["requested_arguments"], &stage.arguments)?;
    arguments(&value["substituted_impl_trait_arguments"], &stage.arguments)?;
    arguments(&value["selected_arguments"], stage.selected_arguments)?;

    let inputs = value["structural_signature"]["inputs"]
        .as_array()
        .ok_or("Missing scoped u16 structural inputs")?;
    let [input] = inputs.as_slice() else {
        return Err("Scoped u16 obligation changes its input arity".into());
    };
    arguments(input, &["u16"])?;
    output(
        &value["structural_signature"]["output"],
        stage.result,
        registry,
    )?;

    display_signature(&value["signature"], stage.result)
}

fn arguments(value: &Value, names: &[&str]) -> Result<(), String> {
    let arguments = value
        .as_array()
        .ok_or("Missing scoped u16 structural arguments")?;
    crate::source_evidence::exact_pairs(
        arguments.iter(),
        names.iter(),
        "Scoped u16 obligation changes its exact substitution arity",
        |argument, name| {
            if identity(argument)? != &builtin(name) {
                return Err("Scoped u16 obligation changes its concrete builtin identity".into());
            }
            Ok(())
        },
    )
}

fn identity(argument: &Value) -> Result<&Value, String> {
    if argument["kind"] != "type"
        || argument["display_only"] != true
        || argument["identity_status"] != "known"
        || argument["value"].as_str().is_none_or(str::is_empty)
    {
        return Err("Scoped u16 obligation lacks known structural type evidence".into());
    }
    argument
        .get("identity")
        .ok_or_else(|| "Missing scoped u16 structural identity".into())
}

fn output(value: &Value, result: bool, registry: &Registry) -> Result<(), String> {
    if !result {
        return arguments(value, &["usize"]);
    }

    let values = value
        .as_array()
        .ok_or("Missing scoped u16 structural output")?;
    let [argument] = values.as_slice() else {
        return Err("Scoped u16 obligation changes its output arity".into());
    };
    let output = identity(argument)?;
    if output["kind"] != "adt"
        || output["arguments"] != Value::Array(["usize", "!"].map(builtin).into())
    {
        return Err("Scoped u16 conversion must return the exact usize/never Result".into());
    }

    selection::pinned_definition(
        &output["definition"],
        ("std::result::Result", "core::result::Result"),
        registry,
    )
}

fn display_signature(value: &Value, result: bool) -> Result<(), String> {
    let output = if result {
        "core::result::Result<usize, !>"
    } else {
        "usize"
    };
    if value["display_only"] != true
        || value["inputs"] != json!(["u16"])
        || value["output"]
            .as_str()
            .map(|text| text.replace("std::", "core::"))
            .as_deref()
            != Some(output)
    {
        return Err("Scoped u16 display signature disagrees with structural proof".into());
    }
    Ok(())
}

fn original_source(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let original = crate::source_evidence::original_expression(node, facts)?;
    let operand = crate::source_evidence::primitive_conversion(
        crate::data_aggregates::ungroup(&original),
        "usize",
    )?;

    crate::source_evidence::verify_child_source(
        crate::data_aggregates::ungroup(operand),
        &node.children[0],
        node,
        facts,
    )
}

/// Structural endpoints share one exact builtin vocabulary across substitutions and results.
fn builtin(name: &str) -> Value {
    crate::structural_json::object([
        ("kind", Value::from("builtin")),
        ("name", Value::from(name)),
    ])
}

fn same_path(actual: &str, expected: &str) -> bool {
    actual.replace("std::", "core::") == expected.replace("std::", "core::")
}

#[cfg(test)]
mod tests;
