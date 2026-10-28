//! Bounded review rosters share original owner resolution and ambiguity checks.
use super::{Locator, local, local_adapter, owner_population::Owner, workflow};
use crate::clones::{FunctionKey, report};
use std::collections::{BTreeMap, BTreeSet};

const MAX_IDENTITIES: usize = 4096;
type Identity = (Locator, local::BuildContext);

pub(super) trait Observation: Owner {
    fn key(&self) -> FunctionKey;
    fn set_unique(&mut self, unique: bool);
}

pub(super) struct Collected<R> {
    pub rows: Vec<R>,
    pub truncated: bool,
    pub unavailable_scope: bool,
}

pub(super) fn collect<S, R: Observation>(
    reviews: impl Iterator<Item = (FunctionKey, S)>,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    mut row: impl FnMut(S, Result<Identity, String>) -> R,
) -> Collected<R> {
    let records: BTreeMap<_, _> = ctx
        .handoff
        .assessed
        .iter()
        .take(MAX_IDENTITIES)
        .map(|record| (record.key, record))
        .collect();
    let mut reviews = reviews;
    let mut rows: Vec<_> = reviews
        .by_ref()
        .take(MAX_IDENTITIES)
        .map(|(key, source)| {
            let identity = identity(key, records.get(&key).copied(), inputs, ctx);
            row(source, identity)
        })
        .collect();
    let truncated = reviews.next().is_some();
    mark_ambiguous(&mut rows, truncated);

    Collected {
        rows,
        truncated,
        unavailable_scope: !ctx.capture.failures.is_empty() || !ctx.handoff.failures.is_empty(),
    }
}

fn mark_ambiguous<R: Observation>(rows: &mut [R], truncated: bool) {
    let mut owners = BTreeMap::<_, Vec<FunctionKey>>::new();
    for row in rows.iter() {
        if let Ok((locator, context)) = row.identity() {
            owners
                .entry((locator, &context.compilation_target))
                .or_default()
                .push(row.key());
        }
    }
    let ambiguous: BTreeSet<_> = owners
        .values()
        .filter(|keys| keys.len() > 1)
        .flatten()
        .copied()
        .collect();

    for row in rows {
        row.set_unique(!truncated && !ambiguous.contains(&row.key()));
    }
}

fn identity(
    key: FunctionKey,
    record: Option<&crate::clones::Function>,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Result<Identity, String> {
    let record = record.ok_or("Missing original review owner")?;
    let unit = workflow::unit(ctx.handoff, ctx.capture, record)?;
    local_adapter::member_key(local_adapter::Input {
        facts: &record.facts,
        unit,
        plan: &ctx.capture.plan,
        index: inputs.indices.get(key.unit())?,
    })
}
