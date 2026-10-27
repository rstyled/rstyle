//! Exact concrete substitution, not display-signature interpretation.
use super::*;

pub(super) fn verify(call: &Value, context: &Context<'_>) -> Result<Argument, String> {
    if context.member.crate_name != context.product.crate_name
        || context.member.stable_crate_id != context.product.stable_crate_id
        || !context.member.local
        || !context.product.local
    {
        return Err("Callback member and source product have different compilation owners".into());
    }
    if call["status"] != "resolved" || call["expanded"] != false {
        return Err("Callback member lacks ordinary complete compiler selection".into());
    }

    definition(&call["requested"], context.member)?;
    definition(&call["selected"], context.member)?;
    let requested = call["requested_arguments"]
        .as_array()
        .ok_or("Missing callback requested substitutions")?;
    let selected = call["selected_arguments"]
        .as_array()
        .ok_or("Missing callback selected substitutions")?;
    if requested.len() != 4 || selected.len() != 4 {
        return Err("Callback member requires its complete four substitutions".into());
    }

    for (left, right) in requested.iter().zip(selected) {
        if left["identity_status"] != "known"
            || right["identity_status"] != "known"
            || left["identity"] != right["identity"]
            || left["kind"] != right["kind"]
        {
            return Err("Callback requested and selected structural substitutions disagree".into());
        }
    }
    if requested[0]["kind"] != "lifetime"
        || requested[0]["identity"]["kind"] != "erased_region"
        || requested[1..].iter().any(|item| item["kind"] != "type")
    {
        return Err("Callback substitutions differ from original lifetime/type parameters".into());
    }

    let argument = scalar(&requested[1]["identity"])?;
    nonzero(&requested[2]["identity"], argument, context)?;
    function(&requested[3]["identity"], argument, context)?;
    Ok(argument)
}

fn scalar(value: &Value) -> Result<Argument, String> {
    if exact_keys(value, &["kind", "name"]) && value["kind"] == "builtin" {
        match value["name"].as_str() {
            Some("u8") => return Ok(Argument::U8),
            Some("u16") => return Ok(Argument::U16),
            _ => {}
        }
    }
    Err("Callback requires a bounded unsigned scalar substitution".into())
}

fn definition(value: &Value, expected: &Definition) -> Result<(), String> {
    let actual: Definition =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if &actual != expected
        || actual.definition_identity.is_empty()
        || actual.stable_crate_id.is_empty()
    {
        return Err("Callback exact compiler definition identity disagrees".into());
    }
    Ok(())
}

fn nonzero(value: &Value, argument: Argument, context: &Context<'_>) -> Result<(), String> {
    let arguments = adt(value, context.nonzero)?;
    if arguments.len() != 1 || scalar(&arguments[0])? != argument {
        return Err("Callback NonZero scalar substitution disagrees".into());
    }
    Ok(())
}

fn adt<'a>(value: &'a Value, expected: &Definition) -> Result<&'a [Value], String> {
    if !exact_keys(value, &["kind", "definition", "arguments"]) || value["kind"] != "adt" {
        return Err("Callback requires a complete structural nominal identity".into());
    }
    definition(&value["definition"], expected)?;
    value["arguments"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "Missing callback structural arguments".into())
}

fn function(value: &Value, argument: Argument, context: &Context<'_>) -> Result<(), String> {
    if !exact_keys(value, &["kind", "definition", "arguments", "signature"])
        || value["kind"] != "function_item"
    {
        return Err("Callback requires an exact function item, not coercion or closure".into());
    }
    definition(&value["definition"], context.constructor)?;
    let arguments = value["arguments"]
        .as_array()
        .ok_or("Missing callback function substitutions")?;
    if arguments.len() != 1 || scalar(&arguments[0])? != argument {
        return Err("Callback function item substitution disagrees".into());
    }

    let signature = &value["signature"];
    if !exact_keys(
        signature,
        &[
            "inputs",
            "output",
            "bound_variables",
            "abi",
            "safety",
            "variadic",
        ],
    ) || signature["bound_variables"] != 0
        || signature["abi"] != "\"Rust\""
        || signature["safety"] != "Safe"
        || signature["variadic"] != false
    {
        return Err("Callback function signature ABI/safety/binders disagree".into());
    }

    let inputs = signature["inputs"]
        .as_array()
        .ok_or("Missing callback function inputs")?;
    if inputs.len() != 1 || scalar(&inputs[0])? != argument {
        return Err("Callback function input substitution disagrees".into());
    }
    let output = adt(&signature["output"], context.option)?;
    if output.len() != 1 {
        return Err("Callback function result is not one Option payload".into());
    }
    nonzero(&output[0], argument, context)
}

fn exact_keys(value: &Value, expected: &[&str]) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == expected.len() && object.keys().all(|key| expected.contains(&key.as_str()))
}
