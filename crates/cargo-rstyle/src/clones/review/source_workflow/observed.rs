//! Both source rules share one bounded original owner identity resolution.
mod measurements;
use super::super::{Locator, length_rules, local, local_adapter, source_rules, workflow};
use super::Family;
use crate::clones::{Function, FunctionKey, analysis, report};
use std::collections::{BTreeMap, BTreeSet};

const MAX_IDENTITIES: usize = 4096;
type Identity = (Locator, local::BuildContext);

pub(super) struct Observations {
    pub rows: Vec<Row>,
    pub truncated: bool,
    pub unavailable_scope: bool,
}

pub(super) struct Rule<P> {
    pub visible: bool,
    pub complete: bool,
    pub policy: P,
    error: Option<String>,
}

pub(super) struct Row {
    pub key: FunctionKey,
    pub selected: bool,
    pub parameters: Rule<source_rules::Policy>,
    pub lengths: Rule<length_rules::Policy>,
    unique: bool,
    witness: Result<Witness, String>,
}

enum Witness {
    Identity(Identity),
    Parameter(source_rules::Snapshot),
    Length(length_rules::Snapshot),
    Both(Box<Snapshots>),
}

struct Snapshots {
    parameters: source_rules::Snapshot,
    lengths: length_rules::Snapshot,
}

struct SnapshotRefs<'a> {
    parameters: Option<&'a source_rules::Snapshot>,
    lengths: Option<&'a length_rules::Snapshot>,
}

impl Witness {
    fn snapshots(&self) -> SnapshotRefs<'_> {
        match self {
            Self::Identity(_) => SnapshotRefs {
                parameters: None,
                lengths: None,
            },
            Self::Parameter(snapshot) => SnapshotRefs {
                parameters: Some(snapshot),
                lengths: None,
            },
            Self::Length(snapshot) => SnapshotRefs {
                parameters: None,
                lengths: Some(snapshot),
            },
            Self::Both(snapshots) => SnapshotRefs {
                parameters: Some(&snapshots.parameters),
                lengths: Some(&snapshots.lengths),
            },
        }
    }
}

impl Row {
    fn new(source: analysis::SourceReview<'_>, identity: Result<Identity, String>) -> Self {
        let parameter = measurements::parameter(&source);
        let length = measurements::length(&source);
        let parameters = Rule {
            visible: source.parameter.visible,
            complete: source.parameter.complete,
            policy: source_rules::Policy {
                max_params: source.parameter.assessment.map_or(0, |rule| rule.limit),
                count_self: source.count_self,
            },
            error: parameter.as_ref().err().cloned(),
        };
        let lengths = Rule {
            visible: source.length.visible,
            complete: source.length.complete,
            policy: length_rules::Policy {
                max_lines: source.length.assessment.map_or(0, |rule| rule.limit),
            },
            error: length.as_ref().err().cloned(),
        };
        let witness = identity.map(|identity| {
            measurements::snapshots(
                identity,
                parameter
                    .ok()
                    .flatten()
                    .map(|signature| (parameters.policy.clone(), signature)),
                length
                    .ok()
                    .flatten()
                    .map(|measurement| (lengths.policy.clone(), measurement)),
            )
        });

        Self {
            key: source.key,
            selected: source.selected,
            parameters,
            lengths,
            unique: true,
            witness,
        }
    }

    pub fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        match self.witness.as_ref().map_err(String::as_str)? {
            Witness::Identity((locator, context)) => Ok((locator, context)),
            Witness::Parameter(snapshot) => Ok((&snapshot.locator, &snapshot.context)),
            Witness::Length(snapshot) => Ok((&snapshot.locator, &snapshot.context)),
            Witness::Both(snapshots) => {
                Ok((&snapshots.parameters.locator, &snapshots.parameters.context))
            }
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
    pub(super) fn complete(&self, family: Family) -> bool {
        self.scope_complete()
            && self.rows.iter().all(|row| {
                family.assessment_complete(row)
                    && (!family.visible(row) || row.checked(family).is_ok())
            })
    }

    fn scope_complete(&self) -> bool {
        !self.truncated && !self.unavailable_scope
    }
}

pub(super) fn collect(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Observations {
    let records: BTreeMap<_, _> = ctx
        .handoff
        .assessed
        .iter()
        .take(MAX_IDENTITIES)
        .map(|record| (record.key, record))
        .collect();
    let mut reviews = inputs.analysis.source_reviews();
    let mut rows: Vec<_> = reviews
        .by_ref()
        .take(MAX_IDENTITIES)
        .map(|source| {
            let identity = identity(source.key, records.get(&source.key).copied(), inputs, ctx);
            Row::new(source, identity)
        })
        .collect();
    let truncated = reviews.next().is_some();
    mark_ambiguous(&mut rows, truncated);

    Observations {
        rows,
        truncated,
        unavailable_scope: !ctx.capture.failures.is_empty() || !ctx.handoff.failures.is_empty(),
    }
}

fn mark_ambiguous(rows: &mut [Row], truncated: bool) {
    let mut owners = BTreeMap::<_, Vec<FunctionKey>>::new();
    for row in rows.iter() {
        if let Ok((locator, context)) = row.identity() {
            owners
                .entry((locator, &context.compilation_target))
                .or_default()
                .push(row.key);
        }
    }
    let ambiguous: BTreeSet<_> = owners
        .values()
        .filter(|keys| keys.len() > 1)
        .flatten()
        .copied()
        .collect();

    for row in rows {
        row.unique = !truncated && !ambiguous.contains(&row.key);
    }
}

fn identity(
    key: FunctionKey,
    record: Option<&Function>,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Result<Identity, String> {
    let record = record.ok_or("Missing source review owner")?;
    let unit = workflow::unit(ctx.handoff, ctx.capture, record)?;
    local_adapter::member_key(local_adapter::Input {
        facts: &record.facts,
        unit,
        plan: &ctx.capture.plan,
        index: inputs.indices.get(key.unit())?,
    })
}
