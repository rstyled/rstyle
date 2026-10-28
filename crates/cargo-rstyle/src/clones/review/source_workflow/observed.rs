//! Bounded original identities and already measured parameter signatures.
use super::super::{Locator, local, local_adapter, source_rules, workflow};
use crate::clones::{Function, FunctionKey, analysis, report};
use std::collections::{BTreeMap, BTreeSet};

const MAX_IDENTITIES: usize = 4096;

pub(super) struct Observations {
    pub rows: Vec<Row>,
    pub truncated: bool,
    pub unavailable_scope: bool,
}

pub(super) struct Row {
    pub key: FunctionKey,
    pub selected: bool,
    pub visible: bool,
    pub complete: bool,
    unique: bool,
    witness: Result<Witness, String>,
}

enum Witness {
    Identity {
        locator: Locator,
        context: local::BuildContext,
        policy: source_rules::Policy,
    },
    Finding(source_rules::Snapshot),
}

impl Row {
    pub fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        match self.witness.as_ref().map_err(String::as_str)? {
            Witness::Identity {
                locator, context, ..
            } => Ok((locator, context)),
            Witness::Finding(snapshot) => Ok((&snapshot.locator, &snapshot.context)),
        }
    }

    pub fn checked(&self) -> Result<source_rules::LiveFinding<'_>, String> {
        if !self.complete || !self.unique {
            return Err(
                "Parameter source assessment or original owner uniqueness is unavailable".into(),
            );
        }
        let snapshot = self.snapshot()?;
        source_rules::LiveFinding::checked(snapshot).map_err(|error| format!("{error:?}"))
    }

    pub fn snapshot(&self) -> Result<&source_rules::Snapshot, String> {
        match self.witness.as_ref().map_err(Clone::clone)? {
            Witness::Finding(snapshot) => Ok(snapshot),
            Witness::Identity { .. } => {
                Err("Owner has no completed visible parameter finding".into())
            }
        }
    }

    pub fn policy(&self) -> Option<&source_rules::Policy> {
        match self.witness.as_ref().ok()? {
            Witness::Identity { policy, .. } => Some(policy),
            Witness::Finding(snapshot) => Some(&snapshot.policy),
        }
    }
}

impl Observations {
    pub fn complete(&self) -> bool {
        !self.truncated
            && !self.unavailable_scope
            && self
                .rows
                .iter()
                .all(|row| row.complete && (!row.visible || row.checked().is_ok()))
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

    let mut reviews = inputs.analysis.parameter_reviews();
    let mut rows: Vec<_> = reviews
        .by_ref()
        .take(MAX_IDENTITIES)
        .map(|row| Row {
            key: row.key,
            selected: row.selected,
            visible: row.visible,
            complete: row.complete,
            unique: true,
            witness: witness(&row, records.get(&row.key).copied(), inputs, ctx),
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

fn witness(
    row: &analysis::ParameterReview<'_>,
    record: Option<&Function>,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Result<Witness, String> {
    let record = record.ok_or("Missing parameter review owner")?;
    let unit = workflow::unit(ctx.handoff, ctx.capture, record)?;
    let (locator, context) = local_adapter::member_key(local_adapter::Input {
        facts: &record.facts,
        unit,
        plan: &ctx.capture.plan,
        index: inputs.indices.get(row.key.unit())?,
    })?;

    if !row.visible || !row.complete {
        return Ok(Witness::Identity {
            locator,
            context,
            policy: source_rules::Policy {
                max_params: row.max_params,
                count_self: row.count_self,
            },
        });
    }
    finding(row, locator, context).map(Witness::Finding)
}

fn finding(
    row: &analysis::ParameterReview<'_>,
    locator: Locator,
    context: local::BuildContext,
) -> Result<source_rules::Snapshot, String> {
    let signature = row
        .signature
        .ok_or("Parameter signature measurement is unavailable")?;
    let assessment = row
        .assessment
        .ok_or("Parameter finding assessment is unavailable")?;
    let actual = assessment
        .actual
        .ok_or("Parameter finding count is unavailable")?;

    let snapshot = source_rules::Snapshot {
        versions: local_adapter::versions("fn_params"),
        locator,
        context,
        policy: source_rules::Policy {
            max_params: row.max_params,
            count_self: row.count_self,
        },
        signature: source_rules::Signature {
            parameters: signature.parameters,
            has_self: signature.has_self,
            actual,
        },
    };
    source_rules::LiveFinding::checked(&snapshot).map_err(|error| format!("{error:?}"))?;
    Ok(snapshot)
}
