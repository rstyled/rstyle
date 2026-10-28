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

pub(super) trait NumericObservation: Observation {
    fn findings_complete(&self) -> bool;
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

/// Completed numeric consumers expose their owner roster without another inspection.
pub(super) trait Source {
    type Review<'a>
    where
        Self: 'a;
    fn reviews(&self) -> impl Iterator<Item = Self::Review<'_>>;
    fn key<'a>(review: &Self::Review<'a>) -> FunctionKey
    where
        Self: 'a;
    fn complete(&self) -> bool;
    fn requested(args: &crate::clones::Args) -> bool;
}

pub(super) struct Observations<R> {
    pub rows: Vec<R>,
    pub truncated: bool,
    pub unavailable_scope: bool,
    pub requested: bool,
    complete: bool,
}

impl<R: NumericObservation> Observations<R> {
    pub fn complete(&self) -> bool {
        self.complete
            && !self.truncated
            && !self.unavailable_scope
            && self.rows.iter().all(NumericObservation::findings_complete)
    }
}

pub(super) fn collect_selected<'s, S: Source, R: NumericObservation>(
    source: &'s S,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    mut row: impl FnMut(S::Review<'s>, bool, Result<Identity, String>) -> R,
) -> Observations<R> {
    let selected = selected_owners(ctx);
    let collected = collect(
        source.reviews().map(|source| {
            let key = S::key(&source);
            (key, (key, source))
        }),
        inputs,
        ctx,
        |source, identity| {
            row(
                source.1,
                selected
                    .get(&source.0)
                    .copied()
                    .unwrap_or(ctx.args.changed_files.is_none()),
                identity,
            )
        },
    );
    Observations {
        rows: collected.rows,
        truncated: collected.truncated,
        unavailable_scope: collected.unavailable_scope,
        requested: S::requested(ctx.args),
        complete: source.complete(),
    }
}

fn selected_owners(ctx: &report::Context<'_>) -> BTreeMap<FunctionKey, bool> {
    ctx.handoff
        .assessed
        .iter()
        .take(MAX_IDENTITIES)
        .map(|function| {
            let selected = ctx
                .analysis_config
                .and_then(crate::clones::analysis::Config::changed_files)
                .is_none_or(|selection| {
                    crate::clones::analysis::selected_file(
                        function,
                        selection,
                        &ctx.capture.plan.root,
                    )
                });
            (function.key, selected)
        })
        .collect()
}

impl Source for crate::clones::analysis::effect_reports::Report<'_> {
    type Review<'a>
        = crate::clones::analysis::effect_reports::Review
    where
        Self: 'a;
    fn reviews(&self) -> impl Iterator<Item = Self::Review<'_>> {
        self.reviews()
    }
    fn key<'a>(review: &Self::Review<'a>) -> FunctionKey
    where
        Self: 'a,
    {
        review.key
    }
    fn complete(&self) -> bool {
        self.review_scope_complete()
    }
    fn requested(args: &crate::clones::Args) -> bool {
        args.effect_density
    }
}

impl Source for crate::clones::analysis::mutation_reports::Report<'_> {
    type Review<'a>
        = crate::clones::analysis::mutation_reports::Review<'a>
    where
        Self: 'a;
    fn reviews(&self) -> impl Iterator<Item = Self::Review<'_>> {
        self.reviews()
    }
    fn key<'a>(review: &Self::Review<'a>) -> FunctionKey
    where
        Self: 'a,
    {
        review.key
    }
    fn complete(&self) -> bool {
        self.review_scope_complete()
    }
    fn requested(args: &crate::clones::Args) -> bool {
        args.mutation_rules
    }
}
