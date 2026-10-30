//! Numeric rule measurements reuse completed assessments and carry no source bytes.
use super::{Identity, Witness};
use crate::clones::{
    analysis,
    review::{length_rules, local_adapter, source_rules, spacing_rules},
};

type Parameter = (source_rules::Policy, source_rules::Signature);
type Length = (length_rules::Policy, length_rules::Measurement);
type Spacing = (spacing_rules::Policy, spacing_rules::Measurement);

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
    numeric(
        &source.length,
        |code_lines| length_rules::Measurement { code_lines },
        "Length",
    )
}

pub(super) fn spacing(
    source: &analysis::SourceReview<'_>,
) -> Result<Option<spacing_rules::Measurement>, String> {
    if source.min_statements.is_none() {
        return Ok(None);
    }
    numeric(
        &source.spacing,
        |direct_statements| spacing_rules::Measurement { direct_statements },
        "Spacing",
    )
}

fn numeric<M>(
    review: &analysis::RuleReview<'_>,
    measure: impl FnOnce(usize) -> M,
    family: &str,
) -> Result<Option<M>, String> {
    if !review.visible || !review.complete {
        return Ok(None);
    }
    let actual = review
        .assessment
        .and_then(|rule| rule.actual)
        .ok_or_else(|| format!("{family} finding count is unavailable"))?;
    Ok(Some(measure(actual)))
}

pub(super) fn snapshots(
    identity: Identity,
    parameter: Option<Parameter>,
    length: Option<Length>,
    spacing: Option<Spacing>,
) -> Witness {
    if let Some(parameter) = parameter {
        let following = following(&identity, length, spacing);
        return Witness::Parameter {
            snapshot: parameter_snapshot(identity, parameter),
            following,
        };
    }
    if let Some(length) = length {
        let spacing = spacing.map(|spacing| Box::new(spacing_snapshot(identity.clone(), spacing)));
        return Witness::Length {
            snapshot: length_snapshot(identity, length),
            spacing,
        };
    }
    match spacing {
        Some(spacing) => Witness::Spacing(spacing_snapshot(identity, spacing)),
        None => Witness::Identity(identity),
    }
}

fn following(
    identity: &Identity,
    length: Option<Length>,
    spacing: Option<Spacing>,
) -> Option<Box<super::Following>> {
    if length.is_none() && spacing.is_none() {
        return None;
    }
    Some(Box::new(super::Following {
        lengths: length.map(|length| length_snapshot(identity.clone(), length)),
        spacings: spacing.map(|spacing| spacing_snapshot(identity.clone(), spacing)),
    }))
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

fn spacing_snapshot(
    (locator, context): Identity,
    (policy, measurement): Spacing,
) -> spacing_rules::Snapshot {
    spacing_rules::Snapshot {
        versions: local_adapter::versions("fn_spacing"),
        locator,
        context,
        policy,
        measurement,
    }
}
