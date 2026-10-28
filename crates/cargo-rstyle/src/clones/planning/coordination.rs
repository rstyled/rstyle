//! Shared bounded coordination of declared source footprints across intent families.
use super::{model, overlap};
use crate::clones::analysis::{Completion, Coverage, Execution};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Family {
    Helper,
    Parameter,
    Guard,
}

#[derive(Debug, Serialize)]
struct PlanReference {
    id: usize,
    family: Family,
    local_id: usize,
}

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    schema: &'static str,
    execution: Execution,
    coverage: Coverage,
    expected_plans: Option<usize>,
    plans: Vec<PlanReference>,
    coordination: model::Analysis,
    whole_plan_independence: &'static str,
    stack_order: &'static str,
}

struct Status {
    complete: bool,
    execution: Execution,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            complete: true,
            execution: Execution::Completed,
        }
    }
}

impl Status {
    fn combine(&self, execution: Execution, complete: bool) -> Self {
        Self {
            complete: self.complete && complete && execution == Execution::Completed,
            execution: combine_execution(self.execution, execution),
        }
    }
}

#[derive(Default)]
struct Population<'a> {
    status: Status,
    total: usize,
    references: Vec<PlanReference>,
    footprints: Vec<model::Plan<'a>>,
}

pub(in crate::clones) fn evaluate(
    helpers: &super::Report,
    parameters: Option<&super::parameters::Report>,
    guards: Option<&super::guards::Report>,
) -> Option<Report> {
    if !helpers.requested() && parameters.is_none() && guards.is_none() {
        return None;
    }

    let mut population = Population::default();
    if helpers.requested() {
        population = population.append(
            Family::Helper,
            Status {
                execution: helpers.execution(),
                complete: helpers.complete(),
            },
            helpers.footprints(),
        );
    }
    if let Some(parameters) = parameters {
        population = population.append(
            Family::Parameter,
            Status {
                execution: parameters.execution(),
                complete: parameters.complete(),
            },
            parameters.footprints(),
        );
    }
    if let Some(guards) = guards {
        population = population.append(
            Family::Guard,
            Status {
                execution: guards.execution(),
                complete: guards.complete(),
            },
            guards.footprints(),
        );
    }
    Some(population.finish())
}

impl<'a> Population<'a> {
    fn append(
        mut self,
        family: Family,
        status: Status,
        plans: impl Iterator<Item = model::Plan<'a>>,
    ) -> Self {
        let status = self.status.combine(status.execution, status.complete);
        for plan in plans {
            self.retain(family, plan);
        }
        Self { status, ..self }
    }

    fn retain(&mut self, family: Family, mut plan: model::Plan<'a>) {
        self.total += 1;
        if self.references.len() == model::MAX_PLANS {
            return;
        }
        let reference = PlanReference {
            id: self.references.len(),
            family,
            local_id: plan.id,
        };
        plan.id = reference.id;

        self.references.push(reference);
        self.footprints.push(plan);
    }

    fn finish(self) -> Report {
        let truncated = self.total > model::MAX_PLANS;
        let coordination = overlap::analyze(&self.footprints, self.status.complete && !truncated);
        let overlap_execution =
            if truncated || coordination.execution == model::Execution::BudgetExhausted {
                Execution::BudgetExhausted
            } else {
                Execution::Completed
            };
        let execution = combine_execution(self.status.execution, overlap_execution);
        let complete = execution == Execution::Completed && coordination.coverage_complete;

        Report {
            schema: "rstyle.edit-coordination/1",
            execution,
            coverage: if complete {
                Coverage::Complete
            } else {
                Coverage::Incomplete
            },
            // Incomplete families may have omitted intents beyond their retained roster.
            expected_plans: self.status.complete.then_some(self.total),
            plans: self.references,
            coordination,
            whole_plan_independence: "unknown",
            stack_order: "not_proven",
        }
    }
}

fn combine_execution(first: Execution, second: Execution) -> Execution {
    if first == Execution::Failed || second == Execution::Failed {
        Execution::Failed
    } else if first == Execution::BudgetExhausted || second == Execution::BudgetExhausted {
        Execution::BudgetExhausted
    } else {
        Execution::Completed
    }
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.execution == Execution::Completed && self.coverage == Coverage::Complete
    }
}

impl Report {
    pub(in crate::clones) fn render(&self) -> String {
        format!(
            "\nEdit coordination: {:?} coverage; {} intents across requested families; {} overlap review groups. Whole-plan independence and stack order remain unknown.",
            self.coverage,
            self.plans.len(),
            self.coordination.overlap_groups.len()
        )
    }
}

#[cfg(test)]
mod tests;
