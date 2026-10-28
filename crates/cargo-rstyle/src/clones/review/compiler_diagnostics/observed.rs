//! Use the core evaluator's retained owner indices; never infer owners from positions.
use super::super::{compiler_owners, local_adapter};
use super::rules::{Owner, Snapshot};
use crate::clones::{identity::UnitContext, report};
use rstyle_core::{Evaluation, facts::FnFacts};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

pub(in crate::clones) fn collect(
    ctx: &report::Context<'_>,
    unit: &UnitContext,
    evaluation: &Evaluation,
    policy: &Rc<serde_json::Value>,
) -> Vec<Result<Snapshot, String>> {
    if evaluation.report.diagnostics.is_empty() {
        return Vec::new();
    }
    let names = compiler_owners::names(&evaluation.facts);
    let originals = compiler_owners::originals(ctx, unit.id);
    let captured = &ctx.capture.units[unit.observed_unit];
    let resolver = Resolver {
        ctx,
        captured,
        names: &names,
    };

    let owners: BTreeMap<_, _> = evaluation
        .diagnostic_owners
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|index| {
            let facts = &evaluation.facts[index];
            let resolved = originals
                .get(facts.id.as_str())
                .ok_or_else(|| "Missing compiler diagnostic owner".to_string())
                .and_then(|original| resolver.resolve(facts, &original.facts));
            (index, resolved)
        })
        .collect();
    snapshots(evaluation, &owners, policy)
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
