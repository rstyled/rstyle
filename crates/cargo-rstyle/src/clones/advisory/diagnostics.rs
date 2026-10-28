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
    pub owners: Vec<compiler_diagnostics::observed::Assessed>,
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
    if ctx.args.review_file.is_none()
        && !ctx
            .args
            .compiler_baseline_requested(crate::clones::CompilerBaseline::Diagnostics)
    {
        let (evaluation, facts) = rstyle_core::evaluate_and_retain(facts, config)?;
        return Ok(Completed {
            evaluation,
            facts,
            snapshots: Vec::new(),
            owners: Vec::new(),
        });
    }

    let mut evaluated = if ctx.args.review_status_requested()
        || ctx
            .args
            .compiler_baseline_requested(crate::clones::CompilerBaseline::Diagnostics)
    {
        rstyle_core::evaluate_with_scopes(facts, config)?
    } else {
        rstyle_core::evaluate_with_owners(facts, config)?
    };
    let compiler_diagnostics::observed::Completed { snapshots, owners } =
        compiler_diagnostics::observed::collect(ctx, unit, &mut evaluated, policy);
    Ok(Completed {
        evaluation: evaluated.report,
        facts: evaluated.facts,
        snapshots,
        owners,
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
