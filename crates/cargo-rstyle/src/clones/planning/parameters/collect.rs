use super::{Argument, Caller, Collected, Plan, UnitCensus, regions::Regions};
use crate::clones::{Function, FunctionKey, report};
use rstyle_pdg::{
    consumer_facts::{callers, mutations},
    unit_store::UnitStore,
};
use std::collections::{BTreeMap, BTreeSet};

struct Group<'a> {
    store: &'a UnitStore,
    records: BTreeMap<usize, &'a Function>,
}

struct Collector<'a, 'context> {
    ctx: &'a report::Context<'context>,
    result: Collected,
    remaining: usize,
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
    for (unit, group) in groups(ctx).into_iter().enumerate() {
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
        collector.inspect_group(group, &targets, unit)?;
    }
    Ok(collector.result)
}

fn groups<'a>(ctx: &report::Context<'a>) -> Vec<Group<'a>> {
    let mut groups: Vec<Group<'a>> = Vec::new();
    for record in &ctx.handoff.assessed {
        let store = record.facts.store.as_ref();
        if let Some(group) = groups
            .iter_mut()
            .find(|group| std::ptr::eq(group.store, store))
        {
            group.records.insert(record.facts.index, record);
        } else {
            groups.push(Group {
                store,
                records: BTreeMap::from([(record.facts.index, record)]),
            });
        }
    }
    groups
}

impl Collector<'_, '_> {
    fn allowance(&mut self, population: usize) -> usize {
        let functions = self.remaining.min(population);
        self.remaining -= functions;
        functions
    }

    fn retain_census(&mut self, unit: usize, census: callers::Census) {
        self.result.budget |= census.execution == mutations::Execution::BudgetExhausted;
        self.result.census.push(UnitCensus { unit, census });
    }

    fn inspect_group(
        &mut self,
        group: Group<'_>,
        targets: &[&Function],
        unit: usize,
    ) -> Result<(), String> {
        let session = inspect_session(group.store, self.allowance(group.store.functions().len()))?;
        let mut calls = CallCollector {
            group: &group,
            session: &session,
            regions: Regions::default(),
            ctx: self.ctx,
        };
        for target in targets {
            self.result
                .plans
                .push(calls.plan(self.result.plans.len(), target)?);
        }

        let census = session
            .into_census()
            .ok_or("Caller census was not completed")?;
        self.retain_census(unit, census);
        Ok(())
    }
}

impl CallCollector<'_, '_, '_> {
    fn plan(&mut self, id: usize, target: &Function) -> Result<Plan, String> {
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
                        span: super::regions::absolute(record.facts.fn_span, argument.span)?,
                        formal: argument.formal.clone(),
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

fn inspect_session(store: &UnitStore, functions: usize) -> Result<callers::Session<'_>, String> {
    let mut session = callers::Session::new(
        store,
        callers::Limits {
            functions,
            ..Default::default()
        },
    )?;
    session.inspect_all()?;
    Ok(session)
}
