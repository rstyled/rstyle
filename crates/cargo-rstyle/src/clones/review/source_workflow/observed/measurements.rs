//! Numeric rule measurements reuse completed assessments and carry no source bytes.
use super::{Identity, Witness};
use crate::clones::{
    analysis,
    review::{length_rules, local_adapter, source_rules},
};

type Parameter = (source_rules::Policy, source_rules::Signature);
type Length = (length_rules::Policy, length_rules::Measurement);

pub(super) fn parameter(
    source: &analysis::SourceReview<'_>,
) -> Result<Option<source_rules::Signature>, String> {
    if !source.parameter.visible || !source.parameter.complete {
        return Ok(None);
    }
    let signature = source
        .signature
        .ok_or("Parameter signature measurement is unavailable")?;
    let actual = source
        .parameter
        .assessment
        .and_then(|rule| rule.actual)
        .ok_or("Parameter finding count is unavailable")?;
    Ok(Some(source_rules::Signature {
        parameters: signature.parameters,
        has_self: signature.has_self,
        actual,
    }))
}

pub(super) fn length(
    source: &analysis::SourceReview<'_>,
) -> Result<Option<length_rules::Measurement>, String> {
    if !source.length.visible || !source.length.complete {
        return Ok(None);
    }
    let code_lines = source
        .length
        .assessment
        .and_then(|rule| rule.actual)
        .ok_or("Length finding count is unavailable")?;
    Ok(Some(length_rules::Measurement { code_lines }))
}

pub(super) fn snapshots(
    identity: Identity,
    parameter: Option<Parameter>,
    length: Option<Length>,
) -> Witness {
    match (parameter, length) {
        (None, None) => Witness::Identity(identity),
        (Some(parameter), None) => Witness::Parameter(parameter_snapshot(identity, parameter)),
        (None, Some(length)) => Witness::Length(length_snapshot(identity, length)),
        (Some(parameter), Some(length)) => {
            let duplicate = (identity.0.clone(), identity.1.clone());
            Witness::Both(Box::new(super::Snapshots {
                parameters: parameter_snapshot(identity, parameter),
                lengths: length_snapshot(duplicate, length),
            }))
        }
    }
}

fn parameter_snapshot(
    (locator, context): Identity,
    (policy, signature): Parameter,
) -> source_rules::Snapshot {
    source_rules::Snapshot {
        versions: local_adapter::versions("fn_params"),
        locator,
        context,
        policy,
        signature,
    }
}

fn length_snapshot(
    (locator, context): Identity,
    (policy, measurement): Length,
) -> length_rules::Snapshot {
    length_rules::Snapshot {
        versions: local_adapter::versions("fn_length"),
        locator,
        context,
        policy,
        measurement,
    }
}
