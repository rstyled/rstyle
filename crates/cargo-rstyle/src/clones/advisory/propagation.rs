//! Borrow captured owners and reuse core propagation across unique direct Cargo edges.
use super::{Collected, UnitId, planning};
use crate::clones::{capture, report::Context};
use rstyle_core::{config::Config, facts::FnFacts, graph, rules::Effect};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Completed {
    pub values: Option<BTreeMap<String, Vec<Effect>>>,
    pub resolved: BTreeMap<String, UnitId>,
    pub unresolved: BTreeSet<String>,
    pub complete: bool,
}

pub(super) struct Evaluation {
    pub units: Vec<Completed>,
    pub planning: Option<planning::Captured>,
}

type Routes = BTreeMap<UnitId, BTreeSet<UnitId>>;
type Identities<'a> = BTreeMap<(UnitId, &'a str), Vec<usize>>;

pub(super) fn evaluate(
    ctx: &Context<'_>,
    inputs: &[Collected],
    config: &Config,
) -> Result<Evaluation, String> {
    if inputs.len() < 2 && !ctx.args.compiler_plans {
        return Ok(local(inputs.len()));
    }
    let routes = routes(ctx);
    let rows = selected_rows(ctx, inputs, config)?;
    let identities = identities(&rows);
    let targets = Targets {
        rows: &rows,
        identities: &identities,
        routes: &routes,
    };
    let mut completed: BTreeMap<_, _> = ctx
        .handoff
        .unit_contexts
        .iter()
        .map(|unit| (unit.id, Completed::default()))
        .collect();
    let facts: Vec<_> = rows.iter().map(|(_, facts)| *facts).collect();
    let (graph, resolved) = targets.graph(&facts, &mut completed);
    if !ctx.args.compiler_plans
        && completed
            .values()
            .all(|unit| unit.resolved.is_empty() && unit.unresolved.is_empty())
    {
        return Ok(local(inputs.len()));
    }

    let effects = rstyle_core::all_effects(&facts, &graph, config);
    let complete = resolved && inputs.iter().all(|input| input.unavailable.is_empty());
    let effects = effects_by_unit(&rows, effects);
    let planning = ctx
        .args
        .compiler_plans
        .then(|| planning::Captured::new(ctx, inputs, &rows, graph));
    Ok(Evaluation {
        units: finish(ctx, completed, effects, complete),
        planning,
    })
}

fn selected_rows<'a>(
    ctx: &Context<'_>,
    inputs: &'a [Collected],
    config: &Config,
) -> Result<Vec<(UnitId, &'a FnFacts)>, String> {
    let excluded = rstyle_core::exclusions::Matcher::compile(&config.exclude.0)?;
    Ok(ctx
        .handoff
        .unit_contexts
        .iter()
        .zip(inputs)
        .flat_map(|(unit, input)| input.facts.iter().map(move |facts| (unit.id, facts)))
        .filter(|(_, facts)| !excluded.is_excluded(std::path::Path::new(&facts.path)))
        .collect())
}

pub(super) fn unresolved(
    facts: &[FnFacts],
    missing: BTreeSet<String>,
    resolved: &BTreeMap<String, UnitId>,
) -> Result<BTreeSet<String>, String> {
    Ok(super::unresolved(facts)?
        .into_iter()
        .chain(missing)
        .filter(|callee| !resolved.contains_key(callee))
        .collect())
}

fn local(units: usize) -> Evaluation {
    Evaluation {
        units: (0..units)
            .map(|_| Completed {
                complete: true,
                ..Completed::default()
            })
            .collect(),
        planning: None,
    }
}

type UnitEffects = BTreeMap<UnitId, BTreeMap<String, Vec<Effect>>>;

fn effects_by_unit(rows: &[(UnitId, &FnFacts)], effects: Vec<Vec<Effect>>) -> UnitEffects {
    let mut grouped = UnitEffects::new();
    for ((unit, facts), effects) in rows.iter().zip(effects) {
        grouped
            .entry(*unit)
            .or_default()
            .insert(facts.id.clone(), effects);
    }
    grouped
}

fn finish(
    ctx: &Context<'_>,
    mut completed: BTreeMap<UnitId, Completed>,
    mut effects: UnitEffects,
    complete: bool,
) -> Vec<Completed> {
    ctx.handoff
        .unit_contexts
        .iter()
        .map(|unit| {
            let mut completed = completed
                .remove(&unit.id)
                .expect("Captured propagation unit");
            completed.values = Some(effects.remove(&unit.id).unwrap_or_default());
            completed.complete = complete;
            completed
        })
        .collect()
}

fn identities<'a>(rows: &[(UnitId, &'a FnFacts)]) -> Identities<'a> {
    let mut identities = Identities::new();
    for (index, (unit, facts)) in rows.iter().enumerate() {
        identities
            .entry((*unit, &facts.id))
            .or_default()
            .push(index);
        if let Some(implements) = &facts.implements {
            identities
                .entry((*unit, implements))
                .or_default()
                .push(index);
        }
    }
    identities
}

struct Targets<'a> {
    rows: &'a [(UnitId, &'a FnFacts)],
    identities: &'a Identities<'a>,
    routes: &'a Routes,
}

impl Targets<'_> {
    fn resolve(&self, caller: usize, callee: &str) -> Vec<usize> {
        let unit = self.rows[caller].0;
        let local = self.identities.get(&(unit, callee));
        let candidates: Vec<_> = self
            .routes
            .get(&unit)
            .into_iter()
            .flatten()
            .filter_map(|unit| self.identities.get(&(*unit, callee)))
            .flatten()
            .copied()
            .filter(|index| self.rows[*index].1.id == callee && self.rows[*index].1.sig.is_none())
            .collect();
        match (local, candidates.as_slice()) {
            (Some(local), []) => local.clone(),
            (None, [target]) => vec![*target],
            _ => Vec::new(),
        }
    }

    fn graph(
        &self,
        facts: &[&FnFacts],
        completed: &mut BTreeMap<UnitId, Completed>,
    ) -> (graph::Graph, bool) {
        let mut complete = true;
        let graph = graph::build_resolved(facts, |caller, edge| {
            let targets = self.resolve(caller, &edge.callee);
            complete &= !targets.is_empty();
            let unit = self.rows[caller].0;
            if targets.is_empty() {
                completed
                    .get_mut(&unit)
                    .expect("Captured propagation unit")
                    .unresolved
                    .insert(edge.callee.clone());
            }
            if let Some(target) = targets.iter().find(|target| self.rows[**target].0 != unit) {
                completed
                    .get_mut(&unit)
                    .expect("Captured propagation unit")
                    .resolved
                    .insert(edge.callee.clone(), self.rows[*target].0);
            }
            targets
        });
        (graph, complete)
    }
}

fn routes(ctx: &Context<'_>) -> Routes {
    if ctx.capture.expected_unit_graph["version"] != 1 {
        return Routes::new();
    }
    let Some(units) = ctx.capture.expected_unit_graph["units"].as_array() else {
        return Routes::new();
    };

    let mut observed = BTreeMap::<usize, Vec<UnitId>>::new();
    for unit in &ctx.handoff.unit_contexts {
        let captured = &ctx.capture.units[unit.observed_unit];
        let matching: Vec<_> = units
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                ctx.capture
                    .plan
                    .primary_units
                    .iter()
                    .chain(&ctx.capture.plan.dependency_units)
                    .any(|planned| planned == *candidate)
                    && matches(candidate, captured)
            })
            .map(|(index, _)| index)
            .collect();
        if let [index] = matching.as_slice() {
            observed.entry(*index).or_default().push(unit.id);
        }
    }

    observed
        .iter()
        .filter_map(|(index, owners)| {
            let [owner] = owners.as_slice() else {
                return None;
            };
            let dependencies = units[*index]["dependencies"].as_array()?;
            let targets = dependencies
                .iter()
                .filter_map(|edge| {
                    let index = usize::try_from(edge["index"].as_u64()?).ok()?;
                    match observed.get(&index)?.as_slice() {
                        [target] => Some(*target),
                        _ => None,
                    }
                })
                .collect();
            Some((*owner, targets))
        })
        .collect()
}

fn matches(candidate: &Value, unit: &capture::Unit) -> bool {
    !unit.test
        && candidate["mode"] == "check"
        && candidate["pkg_id"] == unit.target.package
        && candidate["target"]["name"] == unit.target.name
        && candidate["target"]["src_path"] == unit.target.source.to_string_lossy().as_ref()
        && candidate["target"]["kind"].as_array().is_some_and(|kinds| {
            kinds
                .iter()
                .filter_map(Value::as_str)
                .eq(unit.target.kind.iter().map(String::as_str))
        })
        && candidate["features"]
            == serde_json::json!(capture::invocation_features(&unit.invocation))
        && candidate.get("platform").is_some_and(
            |platform| match rstyle_core::cli_args::flag_value(&unit.invocation, "--target") {
                Some(target) => platform.as_str() == Some(target),
                None => platform.is_null(),
            },
        )
}
