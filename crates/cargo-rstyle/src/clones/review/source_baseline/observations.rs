//! Borrow compact measurements for comparison; clone only for requested publication.
use super::super::{
    Locator, length_rules, local, source_rules,
    source_workflow::{Reports, observed::Row, retained},
    spacing_rules, status,
};
use super::model::Finding;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FindingRef<'a> {
    Parameter(&'a source_rules::Snapshot),
    Length(&'a length_rules::Snapshot),
    Spacing(&'a spacing_rules::Snapshot),
}

impl<'a> From<&'a Finding> for FindingRef<'a> {
    fn from(finding: &'a Finding) -> Self {
        match finding {
            Finding::Parameter(value) => Self::Parameter(value),
            Finding::Length(value) => Self::Length(value),
            Finding::Spacing(value) => Self::Spacing(value),
        }
    }
}

impl<'a> FindingRef<'a> {
    pub(super) fn identity(self) -> (&'a Locator, &'a local::BuildContext) {
        match self {
            Self::Parameter(value) => (&value.locator, &value.context),
            Self::Length(value) => (&value.locator, &value.context),
            Self::Spacing(value) => (&value.locator, &value.context),
        }
    }

    pub(super) fn to_owned(self) -> Finding {
        match self {
            Self::Parameter(value) => Finding::Parameter(value.clone()),
            Self::Length(value) => Finding::Length(value.clone()),
            Self::Spacing(value) => Finding::Spacing(value.clone()),
        }
    }

    pub(super) fn assess(self, owner: &Row) -> (status::State, String) {
        if !owner.selected {
            return (
                status::State::NotRequested,
                "Retained source owner is outside the current selected scope".into(),
            );
        }

        let eligible = match self {
            Self::Parameter(_) => owner.parameters.visible || owner.parameters.numeric_negative,
            Self::Length(_) => owner.lengths.visible || owner.lengths.numeric_negative,
            Self::Spacing(_) => owner.spacings.visible || owner.spacings.numeric_negative,
        };
        if matches!(self, Self::Spacing(_))
            && (!owner.spacing_requested || owner.spacings.policy.min_statements == 0)
        {
            return (
                status::State::NotRequested,
                "Spacing source rule was not requested".into(),
            );
        }
        if !eligible {
            return (
                status::State::Unavailable,
                "Suppressed, excluded or unavailable source evidence is not a numeric negative"
                    .into(),
            );
        }
        match self {
            Self::Parameter(previous) => retained::parameter_owner(owner, previous),
            Self::Length(previous) => retained::length_owner(owner, previous),
            Self::Spacing(previous) => retained::spacing_owner(owner, previous),
        }
    }
}

pub(super) fn findings(row: &Row) -> impl Iterator<Item = Result<FindingRef<'_>, String>> {
    [
        row.parameters.visible.then(|| {
            row.checked_parameter()
                .and_then(|_| row.parameter_snapshot())
                .map(FindingRef::Parameter)
        }),
        row.lengths.visible.then(|| {
            row.checked_length()
                .and_then(|_| row.length_snapshot())
                .map(FindingRef::Length)
        }),
        (row.spacing_requested && row.spacings.visible).then(|| {
            row.checked_spacing()
                .and_then(|_| row.spacing_snapshot())
                .map(FindingRef::Spacing)
        }),
    ]
    .into_iter()
    .flatten()
}

pub(super) fn visible<'a>(
    row: &'a Row,
    reviews: &Reports,
) -> impl Iterator<Item = (&'static str, bool, Result<FindingRef<'a>, String>)> {
    let hidden = [
        reviews.parameters.hidden_for(row.key),
        reviews.lengths.hidden_for(row.key),
        reviews.spacings.hidden_for(row.key),
    ];
    let names = ["fn_params", "fn_length", "fn_spacing"];
    let selected = [
        row.parameters.visible,
        row.lengths.visible,
        row.spacing_requested && row.spacings.visible,
    ];
    names
        .into_iter()
        .zip(hidden)
        .zip(selected)
        .filter(|(_, visible)| *visible)
        .zip(findings(row))
        .map(|(((rule_id, hidden), _), finding)| (rule_id, hidden, finding))
}

pub(super) fn export(observations: &super::Observations) -> Result<Vec<Finding>, String> {
    observations
        .rows
        .iter()
        .flat_map(findings)
        .map(|finding| finding.map(FindingRef::to_owned))
        .collect()
}

pub(super) fn complete(observations: &super::Observations) -> bool {
    use super::super::source_workflow::Family;
    [Family::Parameter, Family::Length, Family::Spacing]
        .into_iter()
        .all(|family| observations.complete(family))
}

type Key<'a> = (
    &'static str,
    &'a Locator,
    &'a super::super::CompilationTarget,
);

pub(super) fn previous(
    baseline: Option<&super::Baseline>,
) -> std::collections::BTreeMap<Key<'_>, FindingRef<'_>> {
    baseline
        .into_iter()
        .flat_map(|baseline| &baseline.entries)
        .map(|finding| {
            let (locator, context) = finding.identity();
            (
                (finding.rule_id(), locator, &context.compilation_target),
                FindingRef::from(finding),
            )
        })
        .collect()
}

pub(super) fn current<'a>(
    row: &'a Row,
    reviews: &Reports,
) -> impl Iterator<Item = super::baseline_common::Observation<Key<'a>, FindingRef<'a>>> {
    visible(row, reviews).map(|(rule_id, review_hidden, finding)| {
        super::baseline_common::Observation {
            key: row.key,
            rule_id: Some(rule_id),
            selected: row.selected,
            review_hidden,
            finding: finding.map(|finding| {
                let (locator, context) = finding.identity();
                ((rule_id, locator, &context.compilation_target), finding)
            }),
        }
    })
}
