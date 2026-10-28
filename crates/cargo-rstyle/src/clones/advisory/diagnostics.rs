//! Retain completed core owners only for explicit review requests.
use crate::clones::{
    identity::{UnitContext, UnitId},
    report::Context,
    review::compiler_diagnostics,
};
use rstyle_core::{facts::FnFacts, report::Report};
use std::rc::Rc;

pub(super) struct Completed {
    pub evaluation: Report,
    pub facts: Vec<FnFacts>,
    pub snapshots: Vec<Result<compiler_diagnostics::Snapshot, String>>,
}

pub(super) fn evaluate(
    ctx: &Context<'_>,
    unit: &UnitContext,
    facts: Vec<FnFacts>,
    policy: &Rc<serde_json::Value>,
) -> Result<Completed, String> {
    let config = ctx
        .compiler_advisory_config
        .ok_or("Compiler diagnostic policy was not requested")?;
    if ctx.args.review_file.is_none() && !ctx.args.compiler_baseline_requested() {
        let (evaluation, facts) = rstyle_core::evaluate_and_retain(facts, config)?;
        return Ok(Completed {
            evaluation,
            facts,
            snapshots: Vec::new(),
        });
    }

    let evaluated = rstyle_core::evaluate_with_owners(facts, config)?;
    let snapshots = compiler_diagnostics::observed::collect(ctx, unit, &evaluated, policy);
    Ok(Completed {
        evaluation: evaluated.report,
        facts: evaluated.facts,
        snapshots,
    })
}

pub(super) fn render(unit: UnitId, report: &Report, visible: Option<&[usize]>) -> String {
    report
        .diagnostics
        .iter()
        .enumerate()
        .filter(|(index, _)| visible.is_none_or(|visible| visible.contains(index)))
        .map(|(index, d)| {
            format!(
                "\nCompiler diagnostic unit {} index {}: {}",
                unit.ordinal(),
                index,
                rstyle_core::report::diagnostic_line(d)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}
