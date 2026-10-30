use super::{Argument, Caller, Collected, Plan, UnitCensus, regions::Regions};
use crate::clones::{Function, FunctionKey, report};
use rstyle_pdg::{
    HirFnFacts,
    consumer_facts::{callers, mutations},
};
use std::collections::{BTreeMap, BTreeSet};

struct Group<'a> {
    unit: usize,
    functions: &'a [HirFnFacts],
    records: BTreeMap<usize, &'a Function>,
}

struct Collector<'a, 'context> {
    ctx: &'a report::Context<'context>,
    result: Collected,
    remaining: usize,
    bundle_remaining: usize,
    use_remaining: usize,
}

struct CallCollector<'a, 'unit, 'context> {
    group: &'a Group<'unit>,
    session: &'a callers::Session<'unit>,
    regions: Regions,
    ctx: &'a report::Context<'context>,
}

pub(super) fn evaluate(
    ctx: &report::Context<'_>,
    keys: &[FunctionKey],
) -> Result<Collected, String> {
    let mut collector = Collector {
        ctx,
        bundle_remaining: super::model::MAX_COMPARISONS,
        use_remaining: callers::Limits::default().uses,
        remaining: ctx
            .args
            .max_analysis_functions
            .unwrap_or(callers::Limits::default().functions),
        result: Collected {
            plans: vec![],
            census: vec![],
            budget: keys.len() > super::model::MAX_PLANS,
        },
    };
    let selected: BTreeSet<_> = keys.iter().copied().collect();
    let mut use_rows = callers::Limits::default().uses;

    for group in groups(ctx)? {
        let targets: Vec<_> = group
            .records
            .values()
            .filter(|record| selected.contains(&record.key))
            .take(super::model::MAX_PLANS.saturating_sub(collector.result.plans.len()))
            .copied()
            .collect();
        if targets.is_empty() {
            continue;
        }
        collector.inspect_group(group, &targets, &mut use_rows)?;
    }
    Ok(collector.result)
}

fn groups<'a>(ctx: &report::Context<'a>) -> Result<Vec<Group<'a>>, String> {
    let mut groups: Vec<Group<'a>> = Vec::new();
    for record in &ctx.handoff.assessed {
        let functions = record.facts.functions()?;
        if let Some(group) = groups
            .iter_mut()
            .find(|group| std::ptr::eq(group.functions, functions))
        {
            group.records.insert(record.facts.index, record);
        } else {
            groups.push(Group {
                unit: groups.len(),
                functions,
                records: BTreeMap::from([(record.facts.index, record)]),
            });
        }
    }
    Ok(groups)
}

impl Collector<'_, '_> {
    fn allowance(&mut self, population: usize) -> usize {
        let functions = self.remaining.min(population);
        self.remaining -= functions;
        functions
    }

    fn retain_census(&mut self, unit: UnitCensus) {
        self.result.budget |= unit.census.execution == mutations::Execution::BudgetExhausted
            || unit.parameter_uses.exhausted();
        self.use_remaining -= unit.census.use_entries;
        self.result.census.push(unit);
    }

    fn retain_plan(&mut self, plan: Plan) {
        self.result.budget |= plan.bundles.exhausted();
        self.result.plans.push(plan);
    }

    fn inspect_group(
        &mut self,
        group: Group<'_>,
        targets: &[&Function],
        use_rows: &mut usize,
    ) -> Result<(), String> {
        let functions = self.allowance(group.functions.len());
        let session = inspect_session(group.functions, functions, self.use_remaining)?;
        self.retain_plans(&group, &session, targets)?;
        let parameter_uses = super::uses::evaluate(&group.records, &session, targets, use_rows)?;

        let census = session
            .into_census()
            .ok_or("Caller census was not completed")?;
        self.retain_census(UnitCensus {
            unit: group.unit,
            census,
            parameter_uses,
        });
        Ok(())
    }

    fn retain_plans(
        &mut self,
        group: &Group<'_>,
        session: &callers::Session<'_>,
        targets: &[&Function],
    ) -> Result<(), String> {
        let mut calls = CallCollector {
            group,
            session,
            regions: Regions::default(),
            ctx: self.ctx,
        };
        for target in targets {
            let plan = calls.plan(self.result.plans.len(), target, &mut self.bundle_remaining)?;
            self.retain_plan(plan);
        }
        Ok(())
    }
}

impl CallCollector<'_, '_, '_> {
    fn plan(
        &mut self,
        id: usize,
        target: &Function,
        bundle_remaining: &mut usize,
    ) -> Result<Plan, String> {
        let signature = self.regions.signature(target, self.ctx)?;
        let mut plan = Plan::new(id, target.key, signature);

        if self
            .session
            .census()
            .is_some_and(|census| census.coverage == mutations::Coverage::Incomplete)
        {
            plan.reasons.push(
                "Caller census is incomplete; inspect the defining unit caller_census reasons"
                    .into(),
            );
        }
        self.gather(&mut plan, target.facts.index)?;
        plan.bundles = super::bundles::evaluate(&plan, bundle_remaining);
        Ok(plan)
    }

    fn gather(&mut self, plan: &mut Plan, target: usize) -> Result<(), String> {
        for witness in self.session.witnesses(target) {
            let Some(record) = self.group.records.get(&witness.caller_index) else {
                plan.reasons
                    .push("Witnessed caller is outside the report function roster".into());
                continue;
            };
            let call = self
                .session
                .call(witness)
                .ok_or("Caller witness has no original call")?;
            let region = match self.regions.call(record, call.span, self.ctx)? {
                Ok(region) => region,
                Err(reason) => {
                    plan.reasons.push(reason);
                    continue;
                }
            };
            let arguments = call
                .arguments
                .iter()
                .map(|argument| {
                    Ok(Argument {
                        span: super::regions::absolute(record.facts.view().fn_span, argument.span)?,
                        formal: argument.formal.clone(),
                        unchanged_formal: argument.unchanged_formal.clone(),
                    })
                })
                .collect::<Result<_, String>>()?;
            plan.callers.push(Caller {
                function: record.key.to_string(),
                region,
                arguments,
            });
        }
        Ok(())
    }
}

fn inspect_session(
    roster: &[HirFnFacts],
    functions: usize,
    uses: usize,
) -> Result<callers::Session<'_>, String> {
    let mut session = callers::Session::new(
        roster,
        callers::Limits {
            functions,
            uses,
            ..Default::default()
        },
        true,
    )?;
    session.inspect_all()?;
    Ok(session)
}
