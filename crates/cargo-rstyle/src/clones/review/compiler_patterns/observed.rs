//! Move each prepared graph into one shared immutable review owner.
use super::super::compiler_owners::{names, originals, signature};
use super::super::{exact_rules, local, local_adapter};
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
    pub candidate: &'a Candidate,
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
    context: local::BuildContext,
}

pub(in crate::clones) fn collect(
    ctx: &report::Context<'_>,
    unit: &UnitContext,
    facts: Vec<FnFacts>,
    candidates: &[Candidate],
) -> Vec<Result<Snapshot, String>> {
    if candidates.is_empty() {
        return Vec::new();
    }
    let names = names(&facts);
    let originals = originals(ctx, unit.id);

    let resolver = Resolver {
        ctx,
        unit: &ctx.capture.units[unit.observed_unit],
        names: &names,
        sites: candidates
            .iter()
            .flat_map(|candidate| &candidate.sites)
            .map(|site| site.id.as_str())
            .collect(),
    };
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
                .and_then(|record| resolver.endpoint(&record.facts, facts));
            (id, endpoint)
        })
        .collect();

    let options = crate::mine_options(&ctx.args.mine);
    candidates
        .iter()
        .map(|candidate| snapshot(candidate, &endpoints, &options))
        .collect()
}

struct Resolver<'a, 'b> {
    ctx: &'a report::Context<'b>,
    unit: &'a crate::clones::capture::Unit,
    names: &'a BTreeMap<String, Result<String, String>>,
    sites: BTreeSet<&'a str>,
}

impl Resolver<'_, '_> {
    fn endpoint(
        &self,
        original: &rstyle_pdg::HirFnFacts,
        mut facts: FnFacts,
    ) -> Result<Endpoint, String> {
        let name = self
            .names
            .get(&facts.id)
            .ok_or("Missing compiler locator")?
            .as_ref()
            .map_err(Clone::clone)?
            .clone();
        let identity = local_adapter::Identity::new(original, self.unit, &self.ctx.capture.plan)?;
        let kind = if facts.sig.is_some() {
            "compiler-method"
        } else {
            "compiler-function"
        };

        let signature = signature(original, facts.sig.take());
        let declaration = Rc::new(Declaration {
            locator: identity.locator(name, kind),
            signature,
            implements: facts.implements.take(),
        });
        let body = self.body(&mut facts);

        Ok(Endpoint {
            declaration,
            body,
            context: local_adapter::context(original, self.unit)?,
        })
    }
    fn body(&self, facts: &mut FnFacts) -> Result<Rc<Body>, String> {
        self.sites
            .contains(facts.id.as_str())
            .then(|| facts.pdg.take())
            .flatten()
            .ok_or_else(|| "Compiler review graph was not captured or not a proposal site".into())
            .and_then(|mut graph| {
                for node in &mut graph.nodes {
                    node.line = 0;
                    if let Some(id) = &node.callee_id
                        && let Some(name) = self.names.get(id)
                    {
                        node.callee_id = Some(name.as_ref().map_err(Clone::clone)?.clone());
                    }
                }
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
    let first = candidate
        .sites
        .first()
        .ok_or("Compiler proposal has no sites")?;
    let context = &get(&first.id)?.context;
    let mut sites = candidate
        .sites
        .iter()
        .map(|site| {
            let endpoint = get(&site.id)?;
            if context != &endpoint.context {
                return Err(
                    "Compiler proposal endpoints have different compilation contexts".into(),
                );
            }
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
    sites.sort_by(|a, b| a.declaration.locator.cmp(&b.declaration.locator));
    definitions.sort_by(|a, b| a.locator.cmp(&b.locator));

    let snapshot = Snapshot {
        versions: local_adapter::versions("compiler-patterns"),
        context: context.clone(),
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
