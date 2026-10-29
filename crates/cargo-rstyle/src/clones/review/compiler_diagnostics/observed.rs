//! Use the core evaluator's retained owner indices; never infer owners from positions.
use super::super::{compiler_owners, local_adapter};
use super::{
    demand::Demand,
    rules::{Owner, Snapshot},
};
use crate::clones::{identity::UnitContext, report};
use rstyle_core::{Evaluation, facts::FnFacts};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Default)]
pub(in crate::clones) struct Completed {
    pub snapshots: Vec<Result<Snapshot, String>>,
    pub owners: Vec<Assessed>,
    pub owner_names: BTreeMap<String, usize>,
    pub owner_scope_truncated: bool,
}

pub(in crate::clones) struct Assessed {
    pub(in crate::clones::review) owner: Result<Rc<Owner>, String>,
    pub(in crate::clones) path: String,
    pub(in crate::clones::review) complexity_measured: bool,
    pub(in crate::clones::review) suppression: Option<rstyle_core::suppress::Suppression>,
    pub(in crate::clones::review) findings: BTreeMap<String, usize>,
}

impl Assessed {
    pub(in crate::clones) fn qualified_name(&self) -> Option<&str> {
        self.owner
            .as_ref()
            .ok()
            .map(|owner| owner.locator.qualified_name.as_str())
    }
}

pub(in crate::clones) struct Policy<'a> {
    pub config: &'a Rc<serde_json::Value>,
    pub metrics: &'static str,
}

pub(in crate::clones) struct Request<'a> {
    pub policy: Policy<'a>,
    pub demand: &'a Demand<'a>,
    pub rules: Option<crate::clones::diagnostic_selection::Request>,
}

pub(in crate::clones) fn collect(
    ctx: &report::Context<'_>,
    unit: &UnitContext,
    evaluation: &mut Evaluation,
    request: Request<'_>,
) -> Completed {
    if evaluation.report.diagnostics.is_empty() && evaluation.scopes.is_none() {
        return Completed::default();
    }

    let scopes = evaluation.scopes.take();
    let names = compiler_owners::collect_names(
        evaluation
            .facts
            .iter()
            .chain(scopes.iter().flat_map(|scopes| &scopes.excluded)),
    );

    let originals = compiler_owners::originals(ctx, unit.id);
    let resolver = Resolver {
        ctx,
        captured: &ctx.capture.units[unit.observed_unit],
        names: &names.by_id,
    };
    let resolve = |facts: &FnFacts| {
        originals
            .get(facts.id.as_str())
            .ok_or_else(|| "Missing compiler diagnostic owner".to_string())
            .and_then(|original| resolver.resolve(facts, &original.facts))
    };

    let (owners, owner_scope_truncated) =
        resolve_owners(evaluation, scopes.as_ref(), &request, resolve);
    let snapshots = snapshots(evaluation, &owners, &request.policy);
    let (owners, owner_names) = scopes.map_or_else(
        || (Vec::new(), BTreeMap::new()),
        |scopes| (assessed(evaluation, scopes, owners), names.counts),
    );
    Completed {
        snapshots,
        owners,
        owner_names,
        owner_scope_truncated,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Index {
    Selected(usize),
    Excluded(usize),
}

type Owners = BTreeMap<Index, Result<Rc<Owner>, String>>;

fn needed(
    evaluation: &Evaluation,
    scopes: Option<&rstyle_core::Scopes>,
    request: &Request<'_>,
) -> BTreeSet<Index> {
    let mut needed: BTreeSet<_> = evaluation
        .report
        .diagnostics
        .iter()
        .zip(&evaluation.diagnostic_owners)
        .filter(|(diagnostic, _)| {
            request
                .rules
                .as_ref()
                .is_none_or(|rules| rules.includes(&diagnostic.rule_id))
        })
        .map(|(_, index)| Index::Selected(*index))
        .collect();
    if let Some(scopes) = scopes {
        needed.extend(
            evaluation
                .facts
                .iter()
                .enumerate()
                .filter(|(_, facts)| request.demand.includes(facts))
                .map(|(index, _)| Index::Selected(index)),
        );
        needed.extend(
            scopes
                .excluded
                .iter()
                .enumerate()
                .filter(|(_, facts)| request.demand.includes(facts))
                .map(|(index, _)| Index::Excluded(index)),
        );
    }
    needed
}

fn resolve_owners(
    evaluation: &Evaluation,
    scopes: Option<&rstyle_core::Scopes>,
    request: &Request<'_>,
    resolve: impl Fn(&FnFacts) -> Result<Rc<Owner>, String>,
) -> (Owners, bool) {
    let mut truncated = false;
    let owners = needed(evaluation, scopes, request)
        .into_iter()
        .map(|index| {
            let owner = if request.demand.claim() {
                let facts = match index {
                    Index::Selected(index) => &evaluation.facts[index],
                    Index::Excluded(index) => {
                        &scopes.expect("Retained core scopes").excluded[index]
                    }
                };
                resolve(facts)
            } else {
                truncated = true;
                Err("Compiler diagnostic owner payload budget exhausted".into())
            };
            (index, owner)
        })
        .collect();
    (owners, truncated)
}

fn findings(evaluation: &Evaluation) -> BTreeMap<usize, BTreeMap<String, usize>> {
    let mut findings = BTreeMap::<usize, BTreeMap<String, usize>>::new();
    for (index, owner) in evaluation.diagnostic_owners.iter().enumerate() {
        findings
            .entry(*owner)
            .or_default()
            .insert(evaluation.report.diagnostics[index].rule_id.clone(), index);
    }
    findings
}

fn assessed(evaluation: &Evaluation, scopes: rstyle_core::Scopes, owners: Owners) -> Vec<Assessed> {
    let mut findings = findings(evaluation);
    let mut suppressions: BTreeMap<_, _> = scopes
        .suppressions
        .into_iter()
        .enumerate()
        .filter(|(index, _)| owners.contains_key(&Index::Selected(*index)))
        .collect();
    owners
        .into_iter()
        .map(|(index, owner)| {
            let (facts, suppression, findings) = match index {
                Index::Selected(index) => (
                    &evaluation.facts[index],
                    Some(
                        suppressions
                            .remove(&index)
                            .expect("Retained core suppression"),
                    ),
                    findings.remove(&index).unwrap_or_default(),
                ),
                Index::Excluded(index) => (&scopes.excluded[index], None, BTreeMap::new()),
            };
            Assessed {
                owner,
                path: facts.path.clone(),
                complexity_measured: facts.complexity.is_some(),
                suppression,
                findings,
            }
        })
        .collect()
}

fn snapshots(
    evaluation: &Evaluation,
    owners: &Owners,
    policy: &Policy<'_>,
) -> Vec<Result<Snapshot, String>> {
    evaluation
        .report
        .diagnostics
        .iter()
        .zip(&evaluation.diagnostic_owners)
        .map(|(diagnostic, owner)| {
            let snapshot = Snapshot {
                owner: Rc::clone(
                    owners
                        .get(&Index::Selected(*owner))
                        .ok_or("Compiler diagnostic rule was not requested")?
                        .as_ref()
                        .map_err(Clone::clone)?,
                ),
                versions: local_adapter::versions("compiler-diagnostics"),
                metrics: policy.metrics.into(),
                policy: Rc::clone(policy.config),
                rule_id: diagnostic.rule_id.clone(),
                actual: diagnostic.actual,
                limit: diagnostic.limit,
                details: diagnostic
                    .details
                    .iter()
                    .map(|detail| (detail.label.clone(), detail.count))
                    .collect(),
            };
            super::super::exact_rules::LiveFinding::checked(&snapshot)
                .map_err(|error| format!("Compiler diagnostic review is unavailable: {error:?}"))?;
            Ok(snapshot)
        })
        .collect()
}

struct Resolver<'a, 'b> {
    ctx: &'a report::Context<'b>,
    captured: &'a crate::clones::capture::Unit,
    names: &'a BTreeMap<String, Result<String, String>>,
}

impl Resolver<'_, '_> {
    fn resolve(
        &self,
        facts: &FnFacts,
        original: &rstyle_pdg::HirFnFacts,
    ) -> Result<Rc<Owner>, String> {
        let name = self
            .names
            .get(&facts.id)
            .ok_or("Missing compiler diagnostic locator")?
            .as_ref()
            .map_err(Clone::clone)?;
        let identity =
            local_adapter::Identity::new(original, self.captured, &self.ctx.capture.plan)?;
        let kind = if facts.sig.is_some() {
            "compiler-method"
        } else {
            "compiler-function"
        };
        Ok(Rc::new(Owner {
            locator: identity.locator(name.clone(), kind),
            context: local_adapter::context(original, self.captured)?,
            signature: compiler_owners::signature(original, facts.sig.clone()),
        }))
    }
}
