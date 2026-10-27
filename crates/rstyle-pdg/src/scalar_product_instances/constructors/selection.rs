//! Reuse the original call-instance selection without widening unary conversion schemas.
use super::*;
use crate::{ByteSpan, HirRef, generated_sources::capture::Definition};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    hir: HirRef,
    span: ByteSpan,
    expanded: bool,
    requested: Definition,
    requested_arguments: Vec<Argument>,
    selected: Definition,
    selected_arguments: Vec<Argument>,
    status: String,
    signature: DisplaySignature,
    #[serde(default)]
    compiled_dependency: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Argument {
    kind: String,
    value: String,
    display_only: bool,
    identity_status: String,
    identity: Structural,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Structural {
    ErasedRegion,
    Builtin { name: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DisplaySignature {
    display_only: bool,
    inputs: Vec<String>,
    output: String,
}

pub(super) fn verify(
    observed: &Observation,
    facts: &dyn EvidenceFacts,
    instance: &Instance,
) -> Result<(), String> {
    let row = original(observed, facts)?;
    let call: Call = serde_json::from_value(row.clone())
        .map_err(|error| format!("Invalid scalar constructor original call selection: {error}"))?;
    if call.hir != observed.hir
        || call.span != observed.span
        || call.expanded
        || call.status != "resolved"
        || call.compiled_dependency.is_some()
        || call.requested != observed.constructor
        || call.selected != observed.constructor
    {
        return Err("Scalar constructor differs from its actual original call selection".into());
    }

    arguments(&call.requested_arguments, instance)?;
    arguments(&call.selected_arguments, instance)?;

    if !call.signature.display_only
        || call.signature.inputs.len() != 2
        || call.signature.inputs.iter().any(String::is_empty)
        || call.signature.output.is_empty()
    {
        return Err("Scalar constructor call display inventory is incomplete".into());
    }
    Ok(())
}

fn original<'a>(
    observed: &Observation,
    facts: &'a dyn EvidenceFacts,
) -> Result<&'a serde_json::Value, String> {
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Unknown scalar constructor original call inventory")?;
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Scalar constructor requires a complete original call inventory".into());
    }

    let calls = inventory["calls"]
        .as_array()
        .ok_or("Unknown scalar constructor call roster")?;
    let mut selected = calls.iter().filter(|row| {
        serde_json::from_value::<HirRef>(row["hir"].clone())
            .ok()
            .as_ref()
            == Some(&observed.hir)
            || serde_json::from_value::<ByteSpan>(row["span"].clone()).ok() == Some(observed.span)
    });
    let row = selected
        .next()
        .ok_or("Unknown scalar constructor original call site")?;
    if selected.next().is_some() {
        return Err("Scalar constructor original call selection is ambiguous or duplicated".into());
    }

    Ok(row)
}

fn arguments(arguments: &[Argument], instance: &Instance) -> Result<(), String> {
    let [lifetime, scalar] = arguments else {
        return Err("Scalar constructor selection requires exact lifetime/scalar arguments".into());
    };
    if !known(lifetime)
        || !known(scalar)
        || lifetime.kind != "lifetime"
        || lifetime.value != "erased"
        || !matches!(lifetime.identity, Structural::ErasedRegion)
        || scalar.kind != "type"
        || scalar.value != instance.argument.as_str()
        || !matches!(&scalar.identity, Structural::Builtin { name } if name == instance.argument.as_str())
    {
        return Err(
            "Scalar constructor original substitution differs from its exact instance".into(),
        );
    }
    Ok(())
}

fn known(argument: &Argument) -> bool {
    argument.display_only && argument.identity_status == "known"
}
