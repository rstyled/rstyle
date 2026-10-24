//! Every repeated raw substitution denotes the same complete compiler identity.
use super::*;

pub(super) fn verify(facts: &HirFnFacts, calls: &[Call; 2]) -> Result<(), String> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Transport substitutions lack their compiled local crate")?;
    check(calls, compiled)
}

fn check(calls: &[Call; 2], compiled: &serde_json::Value) -> Result<(), String> {
    let map = arguments(&calls[0])?;
    let error = arguments(&calls[1])?;
    if map[1].identity != error[1].identity || map[2].identity != error[0].identity {
        return Err(
            "Transport repeated payload substitutions have different compiler identities".into(),
        );
    }
    for call in calls {
        for argument in &call.selected_arguments {
            local(&argument.identity, compiled)?;
        }
    }
    constructor(&map[3].identity, &map[0].identity, &map[2].identity)?;

    constructor(&error[3].identity, &error[1].identity, &error[2].identity)
}

fn arguments(call: &Call) -> Result<&[Argument; 4], String> {
    call.selected_arguments
        .as_slice()
        .try_into()
        .map_err(|_| "Transport requires four complete structural substitutions".into())
}

fn local(value: &Structural, compiled: &serde_json::Value) -> Result<(), String> {
    match value {
        Structural::Adt {
            definition,
            arguments,
        } => {
            definition_owner(definition, compiled)?;
            for argument in arguments {
                local(argument, compiled)?;
            }
        }
        Structural::TupleConstructor {
            definition,
            owner,
            arguments,
            signature,
        } => {
            definition_owner(definition, compiled)?;
            definition_owner(owner, compiled)?;
            for argument in arguments {
                local(argument, compiled)?;
            }
            local(&signature.input, compiled)?;
            local(&signature.output, compiled)?;
        }
        Structural::Builtin { .. } => {
            return Err("Transport payload must have an exact local ADT identity".into());
        }
    }
    Ok(())
}

fn definition_owner(definition: &Definition, compiled: &serde_json::Value) -> Result<(), String> {
    if compiled["identity"] != "rstyle.rust.compiled-crate/1"
        || !definition.local
        || compiled["crate_name"].as_str() != Some(&definition.crate_name)
        || compiled["stable_crate_id"].as_str() != Some(&definition.stable_crate_id)
    {
        return Err("Transport local definition belongs to another compiled crate".into());
    }
    Ok(())
}

fn constructor(value: &Structural, input: &Structural, output: &Structural) -> Result<(), String> {
    let Structural::TupleConstructor {
        definition,
        owner,
        signature,
        ..
    } = value
    else {
        return Err("Transport callback lacks its exact tuple constructor identity".into());
    };
    let Structural::Adt {
        definition: output_owner,
        ..
    } = output
    else {
        return Err("Transport constructor output lacks its exact nominal identity".into());
    };
    if owner != output_owner
        || signature.input.as_ref() != input
        || signature.output.as_ref() != output
        || definition.path != owner.path
    {
        return Err(
            "Transport constructor signature or owner changes a substituted compiler identity"
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
