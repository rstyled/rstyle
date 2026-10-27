//! Asymmetric concrete substitutions retain the actual consuming input type.
use super::{
    Argument, Definition, Observation, Original, Selection, SignatureContract, Structural,
};

#[cfg(test)]
mod tests;

pub(in super::super) fn verify(
    observed: &Observation,
    span: &Structural,
    range: &Structural,
) -> Result<(), String> {
    arguments(&observed.receiver_type, &[range])?;
    arguments(&observed.result_type, &[span])?;
    into(&observed.into, span, range)?;
    from(&observed.from, span, range)
}

fn into(value: &Selection, span: &Structural, range: &Structural) -> Result<(), String> {
    for arguments_value in [
        &value.requested_arguments,
        &value.selected_arguments,
        &value.substituted_impl_trait_arguments,
    ] {
        arguments(arguments_value, &[range, span])?;
    }
    if !matches!(value.original_impl_trait_arguments.as_slice(),
        [Original::Parameter { index: 0, name: t }, Original::Parameter { index: 1, name: u }]
        if t == "T" && u == "U")
    {
        return Err("Range Into changes the original blanket T/U forwarding roles".into());
    }
    signatures(value, range, span)
}

fn from(value: &Selection, span: &Structural, range: &Structural) -> Result<(), String> {
    arguments(&value.requested_arguments, &[span, range])?;
    arguments(&value.substituted_impl_trait_arguments, &[span, range])?;
    if !value.selected_arguments.is_empty() {
        return Err("Concrete local Range From implementation has no substitutions".into());
    }

    let [self_type, input] = value.original_impl_trait_arguments.as_slice() else {
        return Err("Local From requires its complete concrete Self/input roles".into());
    };

    original(self_type, span)?;
    original(input, range)?;
    signatures(value, range, span)
}

fn signatures(value: &Selection, input: &Structural, output: &Structural) -> Result<(), String> {
    for signature in [&value.requested_signature, &value.selected_signature] {
        signature.between(
            input,
            output,
            SignatureContract {
                input_error: "Range conversion requires exactly one value input",
                bound_error: "Range conversion introduces unsupported bound variables",
                argument: single_argument,
            },
        )?;
    }
    Ok(())
}

fn single_argument(values: &[Argument], expected: &Structural) -> Result<(), String> {
    arguments(values, &[expected])
}

fn arguments(observed: &[Argument], expected: &[&Structural]) -> Result<(), String> {
    crate::source_evidence::exact_pairs(
        observed.iter(),
        expected.iter(),
        "Range conversion loses its exact ordered type arguments",
        |actual, expected| {
            if actual.kind != "type"
                || !actual.display_only
                || actual.identity_status != "known"
                || !structural(&actual.identity, expected)
            {
                return Err("Range conversion changes its complete structural type".into());
            }
            Ok(())
        },
    )
}

fn original(value: &Original, expected: &Structural) -> Result<(), String> {
    let same = match (value, expected) {
        (
            Original::Adt {
                definition,
                arguments,
            },
            Structural::Adt {
                definition: expected,
                arguments: expected_arguments,
            },
        ) => {
            definition_matches(definition, expected)
                && structural_arguments(arguments, expected_arguments)
        }
        _ => false,
    };
    if !same {
        return Err("Local From original impl roles differ from concrete Self/input".into());
    }
    Ok(())
}

fn structural(actual: &Structural, expected: &Structural) -> bool {
    match (actual, expected) {
        (Structural::Builtin { name: actual }, Structural::Builtin { name: expected }) => {
            actual == expected
        }
        (
            Structural::Adt {
                definition: actual,
                arguments,
            },
            Structural::Adt {
                definition: expected,
                arguments: expected_arguments,
            },
        ) => {
            definition_matches(actual, expected)
                && structural_arguments(arguments, expected_arguments)
        }
        _ => false,
    }
}

fn structural_arguments(actual: &[Structural], expected: &[Structural]) -> bool {
    crate::source_evidence::all_pairs(actual, expected, structural)
}

fn definition_matches(actual: &Definition, expected: &Definition) -> bool {
    if expected.local {
        return actual == expected;
    }
    actual.path == expected.path
        && actual.definition_identity == expected.definition_identity
        && actual.crate_name == expected.crate_name
        && actual.stable_crate_id == expected.stable_crate_id
        && !actual.local
        && expected.crate_hash.is_some()
        && actual
            .crate_hash
            .as_ref()
            .is_none_or(|hash| Some(hash) == expected.crate_hash.as_ref())
}
