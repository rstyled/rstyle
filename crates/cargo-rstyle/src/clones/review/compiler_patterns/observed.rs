//! Move each prepared graph into one shared immutable review owner.
use super::super::{exact_rules, local, local_adapter};
pub(in crate::clones) use super::rules::Snapshot;
use super::rules::{Body, Declaration, Member, Signature};
use crate::clones::{
    identity::{UnitContext, UnitId},
    report,
};
use rstyle_core::{
    candidates::Candidate,
    cluster::Params,
    facts::{FnFacts, NodeKind},
    locator,
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

    candidates
        .iter()
        .map(|candidate| snapshot(candidate, &endpoints))
        .collect()
}

fn originals<'a>(
    ctx: &'a report::Context<'_>,
    unit: UnitId,
) -> BTreeMap<&'a str, &'a crate::clones::Function> {
    ctx.handoff
        .assessed
        .iter()
        .filter(|record| record.key.unit() == unit)
        .filter_map(|record| {
            record
                .facts
                .build
                .frontend_options
                .get(rstyle_core::advisory::KEY)?["facts"]["id"]
                .as_str()
                .map(|id| (id, record))
        })
        .collect()
}

fn names(facts: &[FnFacts]) -> BTreeMap<String, Result<String, String>> {
    let named: Vec<_> = facts
        .iter()
        .map(|facts| (&facts.id, locator::base(facts)))
        .collect();
    let mut counts = BTreeMap::new();
    for (_, name) in &named {
        *counts.entry(name.as_str()).or_insert(0) += 1;
    }
    named
        .iter()
        .map(|(id, name)| {
            let result = if counts[name.as_str()] != 1 || name.contains("{impl#") {
                Err("Compiler review locator is ambiguous or lacks a stable owner".into())
            } else {
                Ok(name.clone())
            };
            ((*id).clone(), result)
        })
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

fn signature(
    original: &rstyle_pdg::HirFnFacts,
    method: Option<rstyle_core::facts::SigFacts>,
) -> Signature {
    let sig = &original.signature;
    Signature {
        parameters: sig.params.iter().map(|param| param.ty.clone()).collect(),
        result: sig.return_ty.clone(),
        method,
        abi: sig.abi.clone(),
        generics: sig
            .generics
            .iter()
            .map(|param| param.name.clone())
            .collect(),
        has_self: sig.has_self,
        is_variadic: sig.is_variadic,
        is_async: sig.is_async,
        is_unsafe: sig.is_unsafe,
        is_const: sig.is_const,
    }
}

fn snapshot(
    candidate: &Candidate,
    endpoints: &BTreeMap<String, Result<Endpoint, String>>,
) -> Result<Snapshot, String> {
    let get = |id: &str| {
        endpoints
            .get(id)
            .ok_or("Compiler proposal endpoint is not observed")?
            .as_ref()
            .map_err(Clone::clone)
    };
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
        policy: Params::default(),
        kind: candidate.kind,
        sites,
        definitions,
    };

    exact_rules::LiveFinding::checked(&snapshot).map_err(|error| format!("{error:?}"))?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests;
