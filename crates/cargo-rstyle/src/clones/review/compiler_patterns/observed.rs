//! Move each prepared graph into one shared immutable review owner.
use super::super::compiler_owners::{names, originals, signature};
use super::super::{exact_rules, local_adapter};
pub(in crate::clones) use super::rules::Snapshot;
use super::rules::{Body, Declaration, Member};
use crate::clones::{
    identity::{UnitContext, UnitId},
    report,
};
use rstyle_core::{
    candidates::Candidate,
    facts::{FnFacts, NodeKind},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

pub(in crate::clones) struct Row<'a> {
    pub unit: UnitId,
    pub index: usize,
    pub id: &'a str,
    pub snapshot: &'a Result<Snapshot, String>,
}

impl Row<'_> {
    pub(super) fn checked(&self) -> Result<exact_rules::LiveFinding<'_, Snapshot>, String> {
        let snapshot = self.snapshot.as_ref().map_err(Clone::clone)?;
        exact_rules::LiveFinding::checked(snapshot).map_err(|error| format!("{error:?}"))
    }
}

impl super::super::positive_workflow::Row for Row<'_> {
    type Snapshot = Snapshot;
    fn snapshot(&self) -> &Result<Snapshot, String> {
        self.snapshot
    }
}

struct Endpoint {
    declaration: Rc<Declaration>,
    body: Result<Rc<Body>, String>,
}

pub(in crate::clones) fn collect(
    ctx: &report::Context<'_>,
    units: &[&UnitContext],
    facts: Vec<FnFacts>,
    candidates: &[Candidate],
) -> Vec<Result<Snapshot, String>> {
    if candidates.is_empty() {
        return Vec::new();
    }
    let names = names(&facts);
    let originals = population_originals(ctx, units);

    let resolver = Resolver::new(ctx, &names, &originals, candidates);
    let needed: BTreeSet<_> = candidates
        .iter()
        .flat_map(|candidate| candidate.sites.iter().chain(&candidate.definitions))
        .map(|site| site.id.as_str())
        .collect();

    let endpoints: BTreeMap<_, _> = facts
        .into_iter()
        .filter(|facts| needed.contains(facts.id.as_str()))
        .map(|facts| {
            let id = facts.id.clone();
            let endpoint = originals
                .get(id.as_str())
                .ok_or_else(|| "Missing compiler review original owner".into())
                .and_then(|(unit, record)| resolver.endpoint(record.facts.view(), facts, unit));
            (id, endpoint)
        })
        .collect();

    let options = crate::mine_options(&ctx.args.mine);
    candidates
        .iter()
        .map(|candidate| snapshot(candidate, &endpoints, &options))
        .collect()
}

fn population_originals<'a>(
    ctx: &'a report::Context<'_>,
    units: &[&'a UnitContext],
) -> BTreeMap<&'a str, (&'a UnitContext, &'a crate::clones::Function)> {
    units
        .iter()
        .flat_map(|unit| {
            originals(ctx, unit.id)
                .into_iter()
                .map(|(id, record)| (id, (*unit, record)))
        })
        .collect()
}

struct Resolver<'a, 'b> {
    ctx: &'a report::Context<'b>,
    calls: CallNames<'a>,
    sites: BTreeSet<&'a str>,
}

/// Resolve review call locators only within the caller's captured compilation edges.
struct CallNames<'a> {
    names: &'a BTreeMap<String, Result<String, String>>,
    owners: BTreeMap<&'a str, UnitId>,
    routes: BTreeMap<UnitId, BTreeSet<UnitId>>,
}

impl CallNames<'_> {
    fn normalize(&self, graph: &mut rstyle_core::facts::Pdg, caller: UnitId) -> Result<(), String> {
        for node in &mut graph.nodes {
            node.line = 0;
            let Some(id) = &node.callee_id else { continue };
            let Some(owner) = self.owners.get(id.as_str()) else {
                continue;
            };
            let linked = *owner == caller
                || self
                    .routes
                    .get(&caller)
                    .is_some_and(|targets| targets.contains(owner));
            if linked && let Some(name) = self.names.get(id) {
                node.callee_id = Some(name.as_ref().map_err(Clone::clone)?.clone());
            }
        }
        Ok(())
    }
}

impl Resolver<'_, '_> {
    fn new<'a, 'b>(
        ctx: &'a report::Context<'b>,
        names: &'a BTreeMap<String, Result<String, String>>,
        originals: &BTreeMap<&'a str, (&UnitContext, &crate::clones::Function)>,
        candidates: &'a [Candidate],
    ) -> Resolver<'a, 'b> {
        Resolver {
            ctx,
            calls: CallNames {
                names,
                owners: originals
                    .iter()
                    .map(|(id, (unit, _))| (*id, unit.id))
                    .collect(),
                routes: crate::clones::advisory::propagation::invocation_routes(ctx),
            },
            sites: candidates
                .iter()
                .flat_map(|candidate| &candidate.sites)
                .map(|site| site.id.as_str())
                .collect(),
        }
    }

    fn endpoint(
        &self,
        original: crate::clones::FactView<'_>,
        mut facts: FnFacts,
        unit: &UnitContext,
    ) -> Result<Endpoint, String> {
        let name = self
            .calls
            .names
            .get(&facts.id)
            .ok_or("Missing compiler locator")?
            .as_ref()
            .map_err(Clone::clone)?
            .clone();
        let captured = &self.ctx.capture.units[unit.observed_unit];
        let identity =
            local_adapter::Identity::new(original.build, captured, &self.ctx.capture.plan)?;
        let kind = if facts.sig.is_some() {
            "compiler-method"
        } else {
            "compiler-function"
        };

        let signature = signature(original.signature, facts.sig.take());
        let declaration = Rc::new(Declaration {
            locator: identity.locator(name, kind),
            signature,
            context: local_adapter::context(original.build, captured)?,
            implements: facts.implements.take(),
        });
        let body = self.body(&mut facts, unit.id);

        Ok(Endpoint { declaration, body })
    }
    fn body(&self, facts: &mut FnFacts, unit: UnitId) -> Result<Rc<Body>, String> {
        self.sites
            .contains(facts.id.as_str())
            .then(|| facts.pdg.take())
            .flatten()
            .ok_or_else(|| "Compiler review graph was not captured or not a proposal site".into())
            .and_then(|mut graph| {
                self.calls.normalize(&mut graph, unit)?;
                let opaque = graph
                    .nodes
                    .iter()
                    .any(|node| matches!(node.kind, NodeKind::Macro | NodeKind::Closure));
                Ok(Rc::new(Body {
                    graph,
                    opaque_source: opaque.then(|| std::mem::take(&mut facts.source)),
                }))
            })
    }
}

fn retained_endpoint<'a>(
    endpoints: &'a BTreeMap<String, Result<Endpoint, String>>,
    id: &str,
) -> Result<&'a Endpoint, String> {
    endpoints
        .get(id)
        .ok_or("Compiler proposal endpoint is not observed")?
        .as_ref()
        .map_err(Clone::clone)
}

fn snapshot(
    candidate: &Candidate,
    endpoints: &BTreeMap<String, Result<Endpoint, String>>,
    options: &rstyle_core::patterns_report::Options,
) -> Result<Snapshot, String> {
    let get = |id: &str| retained_endpoint(endpoints, id);
    let mut sites = candidate
        .sites
        .iter()
        .map(|site| {
            let endpoint = get(&site.id)?;
            Ok(Member {
                declaration: Rc::clone(&endpoint.declaration),
                body: Rc::clone(endpoint.body.as_ref().map_err(Clone::clone)?),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut definitions = candidate
        .definitions
        .iter()
        .map(|site| get(&site.id).map(|endpoint| Rc::clone(&endpoint.declaration)))
        .collect::<Result<Vec<_>, _>>()?;
    sites.sort_by(|a, b| {
        (&a.declaration.locator, &a.declaration.context)
            .cmp(&(&b.declaration.locator, &b.declaration.context))
    });
    definitions.sort_by(|a, b| (&a.locator, &a.context).cmp(&(&b.locator, &b.context)));

    let snapshot = Snapshot {
        versions: local_adapter::versions("compiler-patterns"),
        policy: options.params,
        hide_implements: options.hide_implements.clone(),
        kind: candidate.kind,
        sites,
        definitions,
    };

    exact_rules::LiveFinding::checked(&snapshot).map_err(|error| format!("{error:?}"))?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests;
