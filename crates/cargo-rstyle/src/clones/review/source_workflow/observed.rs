//! Source rules share one bounded original owner identity resolution.
mod measurements;
use super::super::{
    Locator, length_rules, local, owner_observations, owner_population, source_rules,
    spacing_rules, workflow,
};
use super::Family;
use crate::clones::{FunctionKey, analysis, report};

type Identity = (Locator, local::BuildContext);

pub(in crate::clones::review) struct Observations {
    pub rows: Vec<Row>,
    pub truncated: bool,
    pub unavailable_scope: bool,
    pub spacing_requested: bool,
}

pub(in crate::clones::review) struct Rule<P> {
    pub visible: bool,
    pub complete: bool,
    pub evaluated_negative: bool,
    pub policy: P,
    error: Option<String>,
}

impl<P> Rule<P> {
    fn new(review: &analysis::RuleReview<'_>, policy: P, error: Option<String>) -> Self {
        Self {
            visible: review.visible,
            complete: review.complete,
            evaluated_negative: review.complete
                && !review.suppressed
                && review
                    .assessment
                    .is_some_and(|rule| rule.outcome == analysis::Outcome::EvaluatedNoFinding),
            policy,
            error,
        }
    }

    fn measured<M>(&self, measurement: Result<Option<M>, String>) -> Option<(P, M)>
    where
        P: Clone,
    {
        measurement
            .ok()
            .flatten()
            .map(|value| (self.policy.clone(), value))
    }
}

pub(in crate::clones::review) struct Row {
    pub key: FunctionKey,
    pub selected: bool,
    pub parameters: Rule<source_rules::Policy>,
    pub lengths: Rule<length_rules::Policy>,
    pub spacings: Rule<spacing_rules::Policy>,
    pub spacing_requested: bool,
    unique: bool,
    witness: Result<Witness, String>,
}

enum Witness {
    Identity(Identity),
    Parameter {
        snapshot: source_rules::Snapshot,
        following: Option<Box<Following>>,
    },
    Length {
        snapshot: length_rules::Snapshot,
        spacing: Option<Box<spacing_rules::Snapshot>>,
    },
    Spacing(spacing_rules::Snapshot),
}

struct Following {
    lengths: Option<length_rules::Snapshot>,
    spacings: Option<spacing_rules::Snapshot>,
}

struct SnapshotRefs<'a> {
    parameters: Option<&'a source_rules::Snapshot>,
    lengths: Option<&'a length_rules::Snapshot>,
    spacings: Option<&'a spacing_rules::Snapshot>,
}

impl Witness {
    fn snapshots(&self) -> SnapshotRefs<'_> {
        match self {
            Self::Identity(_) => SnapshotRefs {
                parameters: None,
                lengths: None,
                spacings: None,
            },
            Self::Parameter {
                snapshot,
                following,
            } => SnapshotRefs {
                parameters: Some(snapshot),
                lengths: following.as_ref().and_then(|extra| extra.lengths.as_ref()),
                spacings: following.as_ref().and_then(|extra| extra.spacings.as_ref()),
            },
            Self::Length { snapshot, spacing } => SnapshotRefs {
                parameters: None,
                lengths: Some(snapshot),
                spacings: spacing.as_deref(),
            },
            Self::Spacing(snapshot) => SnapshotRefs {
                parameters: None,
                lengths: None,
                spacings: Some(snapshot),
            },
        }
    }
}

impl Row {
    fn new(source: analysis::SourceReview<'_>, identity: Result<Identity, String>) -> Self {
        let parameter = measurements::parameter(&source);
        let length = measurements::length(&source);
        let spacing = measurements::spacing(&source);
        let parameters = Rule::new(
            &source.parameter,
            source_rules::Policy {
                max_params: source.parameter.assessment.map_or(0, |rule| rule.limit),
                count_self: source.count_self,
            },
            parameter.as_ref().err().cloned(),
        );
        let lengths = Rule::new(
            &source.length,
            length_rules::Policy {
                max_lines: source.length.assessment.map_or(0, |rule| rule.limit),
            },
            length.as_ref().err().cloned(),
        );
        let spacings = Rule::new(
            &source.spacing,
            spacing_rules::Policy {
                min_statements: source.min_statements.unwrap_or(0),
            },
            spacing.as_ref().err().cloned(),
        );
        let witness = identity.map(|identity| {
            measurements::snapshots(
                identity,
                parameters.measured(parameter),
                lengths.measured(length),
                spacings.measured(spacing),
            )
        });

        Self {
            key: source.key,
            selected: source.selected,
            parameters,
            lengths,
            spacings,
            spacing_requested: source.min_statements.is_some(),
            unique: true,
            witness,
        }
    }

    pub fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        match self.witness.as_ref().map_err(String::as_str)? {
            Witness::Identity((locator, context)) => Ok((locator, context)),
            Witness::Parameter { snapshot, .. } => Ok((&snapshot.locator, &snapshot.context)),
            Witness::Length { snapshot, .. } => Ok((&snapshot.locator, &snapshot.context)),
            Witness::Spacing(snapshot) => Ok((&snapshot.locator, &snapshot.context)),
        }
    }

    fn ready<P>(&self, rule: &Rule<P>) -> Result<(), String> {
        if !rule.complete || !self.unique {
            return Err("Source assessment or original owner uniqueness is unavailable".into());
        }
        if let Some(error) = &rule.error {
            return Err(error.clone());
        }
        Ok(())
    }

    pub fn checked_parameter(&self) -> Result<source_rules::LiveFinding<'_>, String> {
        self.ready(&self.parameters)?;
        source_rules::LiveFinding::checked(self.parameter_snapshot()?)
            .map_err(|error| format!("{error:?}"))
    }

    fn checked(&self, family: Family) -> Result<(), String> {
        match family {
            Family::Parameter => self.checked_parameter().map(|_| ()),
            Family::Length => self.checked_length().map(|_| ()),
            Family::Spacing => self.checked_spacing().map(|_| ()),
        }
    }

    pub fn parameter_snapshot(&self) -> Result<&source_rules::Snapshot, String> {
        self.snapshot(|snapshots| snapshots.parameters, "parameter")
    }

    pub fn checked_length(&self) -> Result<length_rules::LiveFinding<'_>, String> {
        self.ready(&self.lengths)?;
        length_rules::LiveFinding::checked(self.length_snapshot()?)
            .map_err(|error| format!("{error:?}"))
    }

    pub fn length_snapshot(&self) -> Result<&length_rules::Snapshot, String> {
        self.snapshot(|snapshots| snapshots.lengths, "length")
    }

    pub fn checked_spacing(&self) -> Result<spacing_rules::LiveFinding<'_>, String> {
        if !self.spacing_requested {
            return Err("Spacing source rule was not requested".into());
        }
        self.ready(&self.spacings)?;
        spacing_rules::LiveFinding::checked(self.spacing_snapshot()?)
            .map_err(|error| format!("{error:?}"))
    }

    pub fn spacing_snapshot(&self) -> Result<&spacing_rules::Snapshot, String> {
        self.snapshot(|snapshots| snapshots.spacings, "spacing")
    }

    fn snapshot<'a, S: 'a>(
        &'a self,
        select: impl FnOnce(SnapshotRefs<'a>) -> Option<&'a S>,
        family: &str,
    ) -> Result<&'a S, String> {
        let witness = self.witness.as_ref().map_err(Clone::clone)?;
        select(witness.snapshots())
            .ok_or_else(|| format!("Owner has no completed visible {family} finding"))
    }
}

impl Observations {
    pub(in crate::clones::review) fn complete(&self, family: Family) -> bool {
        !family.requested(self)
            || self.scope_complete()
                && self.rows.iter().all(|row| {
                    family.assessment_complete(row)
                        && (!family.visible(row) || row.checked(family).is_ok())
                })
    }

    fn scope_complete(&self) -> bool {
        !self.truncated && !self.unavailable_scope
    }
}

pub(in crate::clones::review) fn collect(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Observations {
    let collected = owner_observations::collect(
        inputs
            .analysis
            .source_reviews()
            .map(|source| (source.key, source)),
        inputs,
        ctx,
        Row::new,
    );
    Observations {
        rows: collected.rows,
        truncated: collected.truncated,
        unavailable_scope: collected.unavailable_scope,
        spacing_requested: inputs.analysis.spacing_policy().is_some(),
    }
}

impl owner_population::Owner for Row {
    fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        Row::identity(self)
    }
}

impl owner_observations::Observation for Row {
    fn key(&self) -> FunctionKey {
        self.key
    }

    fn set_unique(&mut self, unique: bool) {
        self.unique = unique;
    }
}
