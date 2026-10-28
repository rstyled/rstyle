//! Baseline headers and retained states share exact build context semantics.
use super::{local, local_adapter, status, workflow};
use crate::clones::report;

use super::{
    baseline_io,
    owner_population::{Domain, Owner, Population},
};
use crate::clones::{FunctionKey, analysis::Coverage};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

pub(super) fn contexts(ctx: &report::Context<'_>) -> Result<Vec<local::BuildContext>, String> {
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

pub(super) fn baseline_state(state: status::State, reason: String) -> (&'static str, String) {
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

pub(in crate::clones) trait Inventory: baseline_io::Artifact {
    type Entry;
    const REPORT_SCHEMA: &'static str;
    const INCOMPLETE: &'static str;
    fn same_scope(&self, previous: &Self) -> bool;
    fn replace_entries(&mut self, entries: Vec<Self::Entry>);
}

pub(super) fn build<T: baseline_io::Artifact, E>(
    entries: Vec<E>,
    mut contexts: Vec<local::BuildContext>,
    construct: impl FnOnce(Vec<E>, Vec<local::BuildContext>) -> T,
) -> Result<T, String> {
    contexts.sort();
    contexts.dedup();

    let baseline = construct(entries, contexts);
    baseline.validate()?;
    Ok(baseline)
}

pub(super) fn publication<T: Inventory>(
    mut header: T,
    requested: bool,
    entries: impl FnOnce() -> Result<Vec<T::Entry>, String>,
) -> Result<Option<T>, String> {
    if !requested {
        return Ok(None);
    }
    header.replace_entries(entries()?);
    header.validate()?;
    Ok(Some(header))
}

#[derive(Serialize)]
pub(in crate::clones) struct Report<T> {
    pub schema: &'static str,
    pub coverage: Coverage,
    pub existing_findings: usize,
    pub new_findings: usize,
    pub scope_changed: bool,
    pub(super) current: Vec<Current>,
    pub(super) retained: Vec<Previous>,
    pub reasons: Vec<String>,
    #[serde(skip)]
    pub publication: Option<T>,
}

#[derive(Serialize)]
pub(super) struct Current {
    pub key: FunctionKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<&'static str>,
    pub selected: bool,
    pub review_hidden: bool,
    pub state: &'static str,
}

#[derive(Serialize)]
pub(super) struct Previous {
    entry_index: usize,
    state: &'static str,
    reason: String,
}

pub(super) struct Observation<K, V> {
    pub key: FunctionKey,
    pub rule_id: Option<&'static str>,
    pub selected: bool,
    pub review_hidden: bool,
    pub finding: Result<(K, V), String>,
}

pub(super) fn compare<K: Ord, V: PartialEq>(
    previous: BTreeMap<K, V>,
    observations: impl Iterator<Item = Observation<K, V>>,
) -> Vec<Current> {
    observations
        .map(|row| {
            let state =
                row.finding
                    .map_or("unavailable", |(key, finding)| match previous.get(&key) {
                        Some(old) if *old == finding => "existing",
                        Some(_) => "changed",
                        None => "new",
                    });
            Current {
                key: row.key,
                rule_id: row.rule_id,
                selected: row.selected,
                review_hidden: row.review_hidden,
                state,
            }
        })
        .collect()
}

pub(super) fn retained<'a, R: Owner + 'a, E: 'a>(
    entries: impl Iterator<Item = (&'a super::Locator, &'a local::BuildContext, &'a E)>,
    population: Population<'_, R>,
    domain: Domain,
    assess: impl Fn(&R, &E) -> (status::State, String),
) -> Vec<Previous> {
    entries
        .enumerate()
        .map(|(entry_index, (locator, context, finding))| {
            let (state, reason) = population
                .resolve(locator, context, domain)
                .map_or_else(|assessment| assessment, |owner| assess(owner, finding));
            let (state, reason) = baseline_state(state, reason);
            Previous {
                entry_index,
                state,
                reason,
            }
        })
        .collect()
}

impl<T: Inventory> Report<T> {
    pub(in crate::clones) fn publish(&self, path: &Path, complete: bool) -> Result<(), String> {
        let artifact = self.ready(complete)?;
        baseline_io::publish(path, artifact)
    }

    fn ready(&self, complete: bool) -> Result<&T, String> {
        if !complete {
            return Err(format!(
                "Incomplete analysis cannot publish a {}",
                T::LABEL.to_lowercase()
            ));
        }
        self.publication
            .as_ref()
            .ok_or_else(|| format!("Incomplete {} export", T::LABEL.to_lowercase()))
    }

    pub(in crate::clones) fn gate_fails(&self) -> bool {
        self.new_findings > 0 || self.scope_changed
    }
    pub(in crate::clones) fn permits_success(&self) -> bool {
        self.coverage == Coverage::Complete
    }
    pub(in crate::clones) fn render(&self) -> String {
        format!(
            "\n{}: {:?} coverage; {} existing, {} new selected visible findings; policy/build scope changed: {}.",
            T::LABEL,
            self.coverage,
            self.existing_findings,
            self.new_findings,
            self.scope_changed
        )
    }
}

pub(in crate::clones) fn publish_optional<T: Inventory>(
    report: Option<&Report<T>>,
    path: Option<&Path>,
    complete: bool,
) -> Result<(), String> {
    if let Some(path) = path {
        report
            .ok_or_else(|| format!("{} was not evaluated", T::LABEL))?
            .publish(path, complete)?;
    }
    Ok(())
}

pub(super) fn counts(current: &[Current]) -> (usize, usize) {
    let visible = current
        .iter()
        .filter(|row| row.selected && !row.review_hidden);
    let existing = visible
        .clone()
        .filter(|row| row.state == "existing")
        .count();
    let new = visible.filter(|row| row.state != "existing").count();
    (existing, new)
}

pub(super) struct Assessment {
    pub complete: bool,
    pub current: Vec<Current>,
    pub retained: Vec<Previous>,
    pub export_requested: bool,
}

impl<T: Inventory> Report<T> {
    pub(super) fn evaluate(
        header: T,
        previous: Option<&T>,
        assessment: Assessment,
        entries: impl FnOnce() -> Result<Vec<T::Entry>, String>,
    ) -> Result<Self, String> {
        let (existing_findings, new_findings) = counts(&assessment.current);
        let scope_changed = previous.is_some_and(|previous| !header.same_scope(previous));
        Ok(Self {
            schema: T::REPORT_SCHEMA,
            coverage: if assessment.complete {
                Coverage::Complete
            } else {
                Coverage::Incomplete
            },
            existing_findings,
            new_findings,
            scope_changed,
            current: assessment.current,
            retained: assessment.retained,
            reasons: if assessment.complete {
                Vec::new()
            } else {
                vec![T::INCOMPLETE.into()]
            },
            publication: publication(
                header,
                assessment.complete && assessment.export_requested,
                entries,
            )?,
        })
    }
}
