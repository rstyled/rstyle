//! Explicit CI inventories reuse completed density evidence, never serialized authority.
mod io;
mod model;
use super::{
    density_rules,
    density_workflow::{
        self,
        observed::{Observations, Row},
    },
    local, local_adapter,
    numeric_workflow::Retained,
    owner_population::{Domain, Population},
    status, workflow,
};
use crate::clones::{FunctionKey, analysis::Coverage, report};
pub(in crate::clones) fn load(path: &Path) -> Result<Baseline, String> {
    io::load(path)
}
pub(in crate::clones) use model::Baseline;
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    schema: &'static str,
    coverage: Coverage,
    existing_findings: usize,
    new_findings: usize,
    scope_changed: bool,
    current: Vec<Current>,
    retained: Vec<Previous>,
    reasons: Vec<String>,
    #[serde(skip)]
    publication: Option<Baseline>,
}

#[derive(Serialize)]
struct Current {
    key: FunctionKey,
    selected: bool,
    review_hidden: bool,
    state: &'static str,
}

#[derive(Serialize)]
struct Previous {
    entry_index: usize,
    state: &'static str,
    reason: String,
}

pub(super) fn evaluate(
    observations: &Observations,
    reviews: &density_workflow::Report,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Result<Report, String> {
    let header = header(ctx)?;
    let complete = observations.complete() && ctx.complete && inputs.effects.baseline_valid();
    let current = compare(observations, reviews, ctx.density_baseline);
    let visible = current
        .iter()
        .filter(|row| row.selected && !row.review_hidden);
    let existing_findings = visible
        .clone()
        .filter(|row| row.state == "existing")
        .count();
    let new_findings = visible.filter(|row| row.state != "existing").count();

    Ok(Report {
        schema: "rstyle.density-baseline-report/1",
        coverage: if complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        existing_findings,
        new_findings,
        scope_changed: ctx.density_baseline.is_some_and(|baseline| {
            baseline.policy != header.policy
                || baseline.contexts != header.contexts
                || baseline.exclude != header.exclude
        }),
        current,
        retained: ctx
            .density_baseline
            .map_or_else(Vec::new, |baseline| retained(baseline, observations)),
        reasons: if complete {
            Vec::new()
        } else {
            vec!["Current density evidence, capture, identity or source policy is incomplete; a baseline cannot establish clean analysis".into()]
        },
        publication: publication(
            header,
            observations,
            ctx.args.write_density_baseline.is_some() && complete,
        )?,
    })
}

fn compare(
    observations: &Observations,
    reviews: &density_workflow::Report,
    baseline: Option<&Baseline>,
) -> Vec<Current> {
    let previous: BTreeMap<_, _> = baseline
        .into_iter()
        .flat_map(|baseline| &baseline.entries)
        .map(|snapshot| {
            (
                (&snapshot.locator, &snapshot.context.compilation_target),
                snapshot,
            )
        })
        .collect();

    observations
        .rows
        .iter()
        .filter(|row| row.visible)
        .map(|row| Current {
            key: row.key,
            selected: row.selected,
            review_hidden: reviews.hidden_for(row.key),
            state: current(row, &previous),
        })
        .collect()
}

fn header(ctx: &report::Context<'_>) -> Result<Baseline, String> {
    let settings = ctx
        .effect_config
        .ok_or("Density baseline requires density policy")?;
    Baseline::new(
        Vec::new(),
        density_rules::Policy::from_config(&settings.side_effects),
        contexts(ctx)?,
        settings.exclude.clone(),
    )
}

fn publication(
    mut header: Baseline,
    observations: &Observations,
    requested: bool,
) -> Result<Option<Baseline>, String> {
    if !requested {
        return Ok(None);
    }
    header.entries = observations
        .rows
        .iter()
        .filter(|row| row.visible)
        .map(|row| row.snapshot().cloned())
        .collect::<Result<_, _>>()?;
    header.validate()?;
    Ok(Some(header))
}

impl Report {
    fn publication(&self, complete: bool) -> Result<&Baseline, String> {
        if !complete {
            return Err("Incomplete analysis cannot publish a density baseline".into());
        }
        self.publication
            .as_ref()
            .ok_or("Incomplete density baseline export".into())
    }

    pub(super) fn publish(&self, path: &Path, complete: bool) -> Result<(), String> {
        io::publish(path, self.publication(complete)?)
    }

    pub(super) fn gate_fails(&self) -> bool {
        self.new_findings > 0 || self.scope_changed
    }

    pub(super) fn permits_success(&self) -> bool {
        self.coverage == Coverage::Complete
    }

    pub(super) fn render(&self) -> String {
        format!(
            "\nDensity baseline: {:?} coverage; {} existing, {} new selected visible findings; policy/build scope changed: {}.",
            self.coverage, self.existing_findings, self.new_findings, self.scope_changed
        )
    }
}

type PreviousIndex<'a> =
    BTreeMap<(&'a super::Locator, &'a super::CompilationTarget), &'a density_rules::Snapshot>;

fn current(row: &Row, previous: &PreviousIndex<'_>) -> &'static str {
    if row.checked().is_err() {
        return "unavailable";
    }
    let Ok(snapshot) = row.snapshot() else {
        return "unavailable";
    };
    match previous.get(&(&snapshot.locator, &snapshot.context.compilation_target)) {
        Some(old) if **old == *snapshot => "existing",
        Some(_) => "changed",
        None => "new",
    }
}

fn retained(baseline: &Baseline, observations: &Observations) -> Vec<Previous> {
    let population = Population::new(
        &observations.rows,
        observations.unavailable_scope,
        observations.truncated,
    );
    baseline
        .entries
        .iter()
        .enumerate()
        .map(|(entry_index, snapshot)| {
            let (state, reason) = population
                .resolve(&snapshot.locator, &snapshot.context, Domain::Density)
                .map_or_else(|assessment| assessment, |owner| owner.assess(snapshot));
            let (state, reason) = baseline_state(state, reason);
            Previous {
                entry_index,
                state,
                reason,
            }
        })
        .collect()
}

fn contexts(ctx: &report::Context<'_>) -> Result<Vec<local::BuildContext>, String> {
    let mut contexts = ctx
        .handoff
        .assessed
        .iter()
        .take(4096)
        .map(|function| {
            local_adapter::context(
                &function.facts,
                workflow::unit(ctx.handoff, ctx.capture, function)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    contexts.sort_unstable();
    contexts.dedup();
    Ok(contexts)
}

fn baseline_state(state: status::State, reason: String) -> (&'static str, String) {
    match state {
        status::State::HiddenExact | status::State::HiddenGrowth => (
            "existing",
            "Exact baseline finding remains current; raw findings are retained".into(),
        ),
        status::State::NoFinding => (
            "resolved",
            "Completed selected numeric negative under the retained policy".into(),
        ),
        status::State::Stale => ("changed", reason),
        status::State::ContextChanged => ("context_changed", reason),
        status::State::NotObserved => ("unobserved", reason),
        status::State::NotRequested => ("unselected", reason),
        status::State::Unavailable => ("unavailable", reason),
    }
}
