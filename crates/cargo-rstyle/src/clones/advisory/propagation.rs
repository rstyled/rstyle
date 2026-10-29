//! Borrow captured owners and reuse core propagation across unique direct Cargo edges.
mod owners;
use super::{Collected, UnitId, planning};
use crate::clones::{capture, report::Context};
use owners::{Owner, Row};
use rstyle_core::{config::Config, facts::FnFacts, graph, rules::Effect};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Completed {
    pub values: Option<BTreeMap<String, Vec<Effect>>>,
    pub resolved: BTreeMap<String, UnitId>,
    pub unresolved: BTreeSet<String>,
    pub complete: bool,
    pub generated: BTreeMap<String, Vec<Effect>>,
    pub local: BTreeSet<String>,
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
    let generated = inputs
        .iter()
        .any(|input| owners::generated(input).next().is_some());
    if inputs.len() < 2 && !ctx.args.compiler_plans && !generated {
        return Ok(local(inputs.len()));
    }
    let routes = routes(ctx);
    let rows = selected_rows(ctx, inputs, config)?;
    let identities = identities(&rows)?;
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
    let graph = targets.graph(&rows, &mut completed);
    if !generated
        && !ctx.args.compiler_plans
        && completed
            .values()
            .all(|unit| unit.resolved.is_empty() && unit.unresolved.is_empty())
    {
        return Ok(local(inputs.len()));
    }

    let effects = rstyle_core::all_effects(&rows, &graph, config);
    let complete = completeness(ctx, inputs, &completed);
    let effects = effects_by_unit(&rows, effects);
    let planning = ctx
        .args
        .compiler_plans
        .then(|| capture_planning(ctx, inputs, &rows, graph));
    Ok(Evaluation {
        units: finish(ctx, completed, effects, complete),
        planning,
    })
}

fn selected_rows<'a>(
    ctx: &Context<'_>,
    inputs: &'a [Collected],
    config: &Config,
) -> Result<Vec<Row<'a>>, String> {
    let excluded = rstyle_core::exclusions::Matcher::compile(&config.exclude.0)?;
    Ok(ctx
        .handoff
        .unit_contexts
        .iter()
        .zip(inputs)
        .flat_map(|(unit, input)| {
            let authored = input.facts.iter().map(move |facts| Row {
                unit: unit.id,
                owner: Owner::Authored(facts),
            });
            let generated = owners::generated(input).map(move |facts| Row {
                unit: unit.id,
                owner: Owner::Generated(facts),
            });
            authored.chain(generated)
        })
        .filter(|row| !excluded.is_excluded(std::path::Path::new(row.path())))
        .collect())
}

impl Completed {
    pub(super) fn unresolved_callees(
        &mut self,
        facts: &[FnFacts],
    ) -> Result<BTreeSet<String>, String> {
        unresolved(
            facts,
            std::mem::take(&mut self.unresolved),
            &self.resolved,
            &self.local,
        )
    }
}

fn unresolved(
    facts: &[FnFacts],
    missing: BTreeSet<String>,
    resolved: &BTreeMap<String, UnitId>,
    local: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    Ok(super::unresolved(facts)?
        .into_iter()
        .chain(missing)
        .filter(|callee| !resolved.contains_key(callee) && !local.contains(callee))
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
struct Effects {
    authored: UnitEffects,
    generated: UnitEffects,
}

fn effects_by_unit(rows: &[Row<'_>], effects: Vec<Vec<Effect>>) -> Effects {
    let mut authored = UnitEffects::new();
    let mut generated = UnitEffects::new();
    for (row, effects) in rows.iter().zip(effects) {
        let grouped = if row.authored().is_some() {
            &mut authored
        } else {
            &mut generated
        };
        grouped
            .entry(row.unit)
            .or_default()
            .insert(row.id().to_owned(), effects);
    }
    Effects {
        authored,
        generated,
    }
}

fn completeness(
    ctx: &Context<'_>,
    inputs: &[Collected],
    completed: &BTreeMap<UnitId, Completed>,
) -> BTreeMap<UnitId, bool> {
    let available: BTreeMap<_, _> = ctx
        .handoff
        .unit_contexts
        .iter()
        .zip(inputs)
        .map(|(unit, input)| {
            (
                unit.id,
                input.unavailable.is_empty() && completed[&unit.id].unresolved.is_empty(),
            )
        })
        .collect();
    if !inputs
        .iter()
        .any(|input| owners::generated(input).next().is_some())
    {
        let complete = available.values().all(|value| *value);
        return available.keys().map(|unit| (*unit, complete)).collect();
    }
    available
        .keys()
        .map(|unit| {
            let mut pending = vec![*unit];
            let mut seen = BTreeSet::new();
            while let Some(dependency) = pending.pop() {
                if !seen.insert(dependency) {
                    continue;
                }
                if !available[&dependency] {
                    return (*unit, false);
                }
                pending.extend(completed[&dependency].resolved.values().copied());
            }
            (*unit, true)
        })
        .collect()
}

fn finish(
    ctx: &Context<'_>,
    mut completed: BTreeMap<UnitId, Completed>,
    mut effects: Effects,
    complete: BTreeMap<UnitId, bool>,
) -> Vec<Completed> {
    ctx.handoff
        .unit_contexts
        .iter()
        .map(|unit| {
            let mut completed = completed
                .remove(&unit.id)
                .expect("Captured propagation unit");
            completed.values = Some(effects.authored.remove(&unit.id).unwrap_or_default());
            completed.complete = complete[&unit.id];
            completed.generated = effects.generated.remove(&unit.id).unwrap_or_default();
            completed
        })
        .collect()
}

fn identities<'a>(rows: &[Row<'a>]) -> Result<Identities<'a>, String> {
    let mut identities = Identities::new();
    let mut owners = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        if row.id().is_empty() || !owners.insert((row.unit, row.id())) {
            return Err(
                "Compiler advisory has missing or duplicate function identities in one unit".into(),
            );
        }
        identities
            .entry((row.unit, row.id()))
            .or_default()
            .push(index);
        if let Some(implements) = row.implements() {
            identities
                .entry((row.unit, implements))
                .or_default()
                .push(index);
        }
    }
    Ok(identities)
}

struct Targets<'a> {
    rows: &'a [Row<'a>],
    identities: &'a Identities<'a>,
    routes: &'a Routes,
}

impl Targets<'_> {
    fn resolve(&self, caller: usize, callee: &str) -> Vec<usize> {
        let unit = self.rows[caller].unit;
        let local = self.identities.get(&(unit, callee));
        let candidates: Vec<_> = self
            .routes
            .get(&unit)
            .into_iter()
            .flatten()
            .filter_map(|unit| self.identities.get(&(*unit, callee)))
            .flatten()
            .copied()
            .filter(|index| self.rows[*index].id() == callee)
            .collect();
        match (local, candidates.as_slice()) {
            (Some(local), []) => local.clone(),
            (None, [target]) => vec![*target],
            _ => Vec::new(),
        }
    }

    fn graph(
        &self,
        facts: &[Row<'_>],
        completed: &mut BTreeMap<UnitId, Completed>,
    ) -> graph::Graph {
        graph::build_resolved(facts, |caller, edge| {
            let targets = self.resolve(caller, &edge.callee);
            let unit = self.rows[caller].unit;
            if targets.is_empty() {
                completed
                    .get_mut(&unit)
                    .expect("Captured propagation unit")
                    .unresolved
                    .insert(edge.callee.clone());
            }
            if let Some(target) = targets
                .iter()
                .find(|target| self.rows[**target].unit != unit)
            {
                completed
                    .get_mut(&unit)
                    .expect("Captured propagation unit")
                    .resolved
                    .insert(edge.callee.clone(), self.rows[*target].unit);
            }
            if !targets.is_empty() && targets.iter().all(|target| self.rows[*target].unit == unit) {
                completed
                    .get_mut(&unit)
                    .expect("Captured propagation unit")
                    .local
                    .insert(edge.callee.clone());
            }
            targets
        })
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

fn capture_planning(
    ctx: &Context<'_>,
    inputs: &[Collected],
    rows: &[Row<'_>],
    graph: graph::Graph,
) -> planning::Captured {
    let authored: Vec<_> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.authored().map(|facts| (index, (row.unit, facts))))
        .collect();

    let indices: BTreeMap<_, _> = authored
        .iter()
        .enumerate()
        .map(|(projected, (index, _))| (*index, projected))
        .collect();

    let callees = authored
        .iter()
        .map(|(index, _)| {
            graph.callees[*index]
                .iter()
                .filter_map(|call| {
                    let targets: Vec<_> = call
                        .targets
                        .iter()
                        .filter_map(|target| indices.get(target).copied())
                        .collect();
                    (!targets.is_empty()).then(|| graph::Callee {
                        line: call.line,
                        targets,
                        args: call.args.clone(),
                    })
                })
                .collect()
        })
        .collect();
    let facts: Vec<_> = authored.into_iter().map(|(_, row)| row).collect();
    planning::Captured::new(ctx, inputs, &facts, graph::Graph { callees })
}
