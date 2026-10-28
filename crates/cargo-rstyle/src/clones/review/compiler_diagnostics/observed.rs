//! Use the core evaluator's retained owner indices; never infer owners from positions.
use super::super::{compiler_owners, local_adapter};
use super::rules::{Owner, Snapshot};
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
}

pub(in crate::clones) struct Assessed {
    pub(in crate::clones::review) owner: Result<Rc<Owner>, String>,
    pub(in crate::clones) path: String,
    pub(in crate::clones::review) suppression: Option<rstyle_core::suppress::Suppression>,
    pub(in crate::clones::review) findings: BTreeMap<String, usize>,
}

pub(in crate::clones) fn collect(
    ctx: &report::Context<'_>,
    unit: &UnitContext,
    evaluation: &mut Evaluation,
    policy: &Rc<serde_json::Value>,
) -> Completed {
    if evaluation.report.diagnostics.is_empty() && evaluation.scopes.is_none() {
        return Completed::default();
    }

    let scopes = evaluation.scopes.take();
    let names = compiler_owners::names(
        evaluation
            .facts
            .iter()
            .chain(scopes.iter().flat_map(|scopes| &scopes.excluded)),
    );

    let originals = compiler_owners::originals(ctx, unit.id);
    let resolver = Resolver {
        ctx,
        captured: &ctx.capture.units[unit.observed_unit],
        names: &names,
    };
    let resolve = |facts: &FnFacts| {
        originals
            .get(facts.id.as_str())
            .ok_or_else(|| "Missing compiler diagnostic owner".to_string())
            .and_then(|original| resolver.resolve(facts, &original.facts))
    };

    let owners = resolve_owners(evaluation, scopes.is_some(), resolve);
    let snapshots = snapshots(evaluation, &owners, policy);
    let owners = scopes.map_or_else(Vec::new, |scopes| {
        assessed(evaluation, scopes, owners, resolve)
    });
    Completed { snapshots, owners }
}

type Owners = BTreeMap<usize, Result<Rc<Owner>, String>>;

fn resolve_owners(
    evaluation: &Evaluation,
    retain_scopes: bool,
    resolve: impl Fn(&FnFacts) -> Result<Rc<Owner>, String>,
) -> Owners {
    let mut needed: BTreeSet<_> = evaluation.diagnostic_owners.iter().copied().collect();
    if retain_scopes {
        needed.extend(0..evaluation.facts.len().min(4096));
    }
    needed
        .into_iter()
        .map(|index| (index, resolve(&evaluation.facts[index])))
        .collect()
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

fn assessed(
    evaluation: &Evaluation,
    scopes: rstyle_core::Scopes,
    mut owners: Owners,
    resolve: impl Fn(&FnFacts) -> Result<Rc<Owner>, String>,
) -> Vec<Assessed> {
    let mut findings = findings(evaluation);
    let mut assessed: Vec<_> = evaluation
        .facts
        .iter()
        .zip(scopes.suppressions)
        .take(4096)
        .enumerate()
        .map(|(index, (facts, suppression))| Assessed {
            owner: owners.remove(&index).expect("Retained core owner"),
            path: facts.path.clone(),
            suppression: Some(suppression),
            findings: findings.remove(&index).unwrap_or_default(),
        })
        .collect();

    let remaining = 4096 - assessed.len();
    assessed.extend(
        scopes
            .excluded
            .iter()
            .take(remaining)
            .map(|facts| Assessed {
                owner: resolve(facts),
                path: facts.path.clone(),
                suppression: None,
                findings: BTreeMap::new(),
            }),
    );
    assessed
}

fn snapshots(
    evaluation: &Evaluation,
    owners: &BTreeMap<usize, Result<Rc<Owner>, String>>,
    policy: &Rc<serde_json::Value>,
) -> Vec<Result<Snapshot, String>> {
    evaluation
        .report
        .diagnostics
        .iter()
        .zip(&evaluation.diagnostic_owners)
        .map(|(diagnostic, owner)| {
            let snapshot = Snapshot {
                owner: Rc::clone(owners[owner].as_ref().map_err(Clone::clone)?),
                versions: local_adapter::versions("compiler-diagnostics"),
                metrics: rstyle_core::advisory::METRICS.into(),
                policy: Rc::clone(policy),
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
