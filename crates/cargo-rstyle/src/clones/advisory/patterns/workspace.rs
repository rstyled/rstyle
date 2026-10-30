//! Select nonoverlapping compiler populations, then reuse the core proposer.
mod identities;
use super::super::{Ready, Unit, propagation};
use super::{Inputs, Report};
use crate::clones::{identity::UnitId, report::Context};
use identities::{qualify, root};
mod restored;
pub(super) use restored::restore_evaluation;

use std::collections::{BTreeMap, BTreeSet};

struct Population {
    test: bool,
    namespaces: BTreeSet<String>,
    sources: BTreeSet<String>,
    inputs: Vec<(usize, Inputs)>,
}

pub(in crate::clones::advisory) fn finish(
    ctx: &Context<'_>,
    ready: Vec<Ready>,
) -> Result<Vec<Unit>, String> {
    let mut units = Vec::with_capacity(ready.len());
    let mut populations: Vec<Population> = Vec::new();
    for (index, ready) in ready.into_iter().enumerate() {
        let (unit, input) = ready.finish()?;
        units.push(unit);
        if !ctx.args.compiler_patterns {
            continue;
        }
        let captured = &ctx.capture.units[ctx.handoff.unit_contexts[index].observed_unit];
        let namespaces = unit_namespaces(ctx, index);
        let sources = unit_sources(ctx, index);

        let existing = populations.iter_mut().find(|population| {
            population.test == captured.test
                && population.namespaces.is_disjoint(&namespaces)
                && population.sources.is_disjoint(&sources)
        });
        if let Some(population) = existing {
            population.namespaces.extend(namespaces);
            population.sources.extend(sources);
            population.inputs.push((index, input));
        } else {
            populations.push(Population {
                test: captured.test,
                namespaces,
                sources,
                inputs: vec![(index, input)],
            });
        }
    }
    for population in populations {
        let anchor = population.inputs[0].0;
        units[anchor].patterns = Some(evaluate(ctx, population)?);
    }
    Ok(units)
}

fn unit_sources(ctx: &Context<'_>, index: usize) -> BTreeSet<String> {
    crate::clones::review::compiler_owners::originals(ctx, ctx.handoff.unit_contexts[index].id)
        .values()
        .filter_map(|record| {
            record
                .facts
                .view()
                .build
                .frontend_options
                .get(rstyle_core::advisory::KEY)?["facts"]["path"]
                .as_str()
                .map(str::to_owned)
        })
        .collect()
}

fn unit_namespaces(ctx: &Context<'_>, index: usize) -> BTreeSet<String> {
    let unit = &ctx.handoff.unit_contexts[index];
    let captured = &ctx.capture.units[unit.observed_unit];
    let mut names: BTreeSet<_> = crate::clones::review::compiler_owners::originals(ctx, unit.id)
        .into_keys()
        .map(|id| root(id).to_owned())
        .collect();

    if let Some(name) = rstyle_core::cli_args::flag_value(&captured.invocation, "--crate-name") {
        names.insert(name.to_owned());
    }
    names
}

fn evaluate(ctx: &Context<'_>, population: Population) -> Result<Report, String> {
    let members: Vec<_> = population
        .inputs
        .iter()
        .map(|(index, _)| &ctx.handoff.unit_contexts[*index])
        .collect();
    let scoped = members.len() > 1;
    let scopes = scopes(ctx, &population);
    let roots = (0..ctx.handoff.unit_contexts.len())
        .flat_map(|index| unit_namespaces(ctx, index))
        .collect();

    let mut inputs = population.inputs.into_iter();
    let (first, mut combined) = inputs.next().expect("Nonempty compiler population");
    if scoped {
        qualify(&mut combined.facts, members[0].id, &scopes[&first], &roots);
    }
    for (index, mut input) in inputs {
        if scoped {
            qualify(
                &mut input.facts,
                ctx.handoff.unit_contexts[index].id,
                &scopes[&index],
                &roots,
            );
        }
        combined = combined.append(input)?;
    }
    super::evaluate(ctx, &members, combined, scoped)
}

/// A reference can join a foreign owner only through the captured direct Cargo edge.
fn scopes(ctx: &Context<'_>, population: &Population) -> BTreeMap<usize, BTreeMap<String, UnitId>> {
    let routes = propagation::invocation_routes(ctx);
    let names: BTreeMap<_, _> = population
        .inputs
        .iter()
        .map(|(index, _)| {
            (
                ctx.handoff.unit_contexts[*index].id,
                unit_namespaces(ctx, *index),
            )
        })
        .collect();
    population
        .inputs
        .iter()
        .map(|(index, _)| {
            let unit = ctx.handoff.unit_contexts[*index].id;
            let targets =
                std::iter::once(unit).chain(routes.get(&unit).into_iter().flatten().copied());
            let scopes = targets
                .filter_map(|target| names.get(&target).map(|names| (target, names)))
                .flat_map(|(target, names)| names.iter().map(move |name| (name.clone(), target)))
                .collect();
            (*index, scopes)
        })
        .collect()
}
