//! Bind captured selection to the exact source instance and compiled local crate.
use super::*;
use crate::{declaration_origins::Origins, generated_sources::capture::Definition};

pub(super) fn verify(
    observed: &Observation,
    facts: &dyn EvidenceFacts,
    instance: &Instance,
) -> Result<(), String> {
    let argument = GenericArgument::Scalar(instance.argument);
    if observed.arguments != [GenericArgument::ErasedLifetime, argument]
        || observed.owner.path != instance.nominal.definition_path
        || observed.constructor.path != observed.owner.path
        || observed.signature.output_owner != observed.owner
        || observed.signature.output_arguments != observed.arguments
    {
        return Err(
            "Scalar constructor owner or substitution differs from its exact instance".into(),
        );
    }
    definitions(observed, facts)?;
    signature(observed, instance)?;

    declaration(observed, facts, instance)
}

fn definitions(observed: &Observation, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Unknown scalar constructor compiled crate identity")?;
    if compiled.get("identity").and_then(serde_json::Value::as_str)
        != Some("rstyle.rust.compiled-crate/1")
    {
        return Err("Scalar constructor compiled crate identity is unregistered".into());
    }
    let mut identities = std::collections::BTreeSet::new();
    for definition in [
        &observed.constructor,
        &observed.owner,
        &observed.fields[0].definition,
        &observed.fields[1].definition,
    ] {
        definition_matches(definition, compiled)?;
        if !identities.insert(&definition.definition_identity) {
            return Err("Scalar constructor definitions have duplicate compiler identities".into());
        }
    }

    Ok(())
}

fn definition_matches(definition: &Definition, compiled: &serde_json::Value) -> Result<(), String> {
    if !definition.local
        || definition.path.is_empty()
        || definition.definition_identity.is_empty()
        || definition.stable_crate_id.is_empty()
        || compiled
            .get("crate_name")
            .and_then(serde_json::Value::as_str)
            != Some(definition.crate_name.as_str())
        || compiled
            .get("stable_crate_id")
            .and_then(serde_json::Value::as_str)
            != Some(definition.stable_crate_id.as_str())
    {
        return Err("Scalar constructor definition differs from its compiled local crate".into());
    }
    Ok(())
}

fn signature(observed: &Observation, instance: &Instance) -> Result<(), String> {
    let expected = [
        ValueType::SharedByteSlice,
        ValueType::Scalar(instance.argument),
    ];
    if observed.signature.inputs != expected
        || observed.signature.bound_variables != 0
        || observed.signature.abi != "\"Rust\""
        || observed.signature.safety != "Safe"
        || observed.signature.variadic
    {
        return Err("Scalar constructor lacks its exact safe binary Rust signature".into());
    }
    for (position, field) in observed.fields.iter().enumerate() {
        if field.position != position as u32
            || field.ty != expected[position]
            || field.definition.path != format!("{}::{position}", observed.owner.path)
        {
            return Err(
                "Scalar constructor selected fields disagree with original tuple order".into(),
            );
        }
    }
    Ok(())
}

fn declaration(
    observed: &Observation,
    facts: &dyn EvidenceFacts,
    instance: &Instance,
) -> Result<(), String> {
    let origins = facts.build.frontend_options.data::<Origins>()?;
    if origins.identity != crate::declaration_origins::IDENTITY {
        return Err("Scalar constructor declaration origin inventory is unregistered".into());
    }
    let mut matches = origins
        .declarations
        .iter()
        .filter(|origin| origin.nominal == instance.nominal);
    let origin = matches
        .next()
        .ok_or("Unknown scalar constructor declaration origin")?;

    if matches.next().is_some()
        || origin.source_file != observed.declaration.source_file
        || origin.declaration_span != observed.declaration.span
        || crate::source_evidence::byte_range(&origin.file_bytes, origin.declaration_span)?
            != instance.source.as_str()
    {
        return Err(
            "Scalar constructor declaration differs from its exact original source instance".into(),
        );
    }
    Ok(())
}
