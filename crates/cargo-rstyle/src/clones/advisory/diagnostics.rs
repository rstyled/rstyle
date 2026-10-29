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
    pub diagnostic_indices: Vec<usize>,
    pub effects: Option<std::collections::BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    pub snapshots: Vec<Result<compiler_diagnostics::Snapshot, String>>,
    pub owners: Vec<compiler_diagnostics::observed::Assessed>,
}

pub(super) struct Inputs {
    pub facts: Vec<FnFacts>,
    pub effects: Option<std::collections::BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    pub metrics: &'static str,
}

pub(super) fn evaluate(
    ctx: &Context<'_>,
    unit: &UnitContext,
    inputs: Inputs,
    policy: &Rc<serde_json::Value>,
) -> Result<Completed, String> {
    let retention = retention(ctx);
    let config = ctx
        .compiler_advisory_config
        .ok_or("Compiler diagnostic policy was not requested")?;
    if ctx.args.review_file.is_none()
        && !ctx
            .args
            .compiler_baseline_requested(crate::clones::CompilerBaseline::Diagnostics)
    {
        let evaluated = evaluate_core(inputs, config, retention, ctx.args.compiler_patterns)?;
        let (evaluation, facts) = (evaluated.report, evaluated.facts);
        return Ok(Completed {
            evaluation,
            facts,
            diagnostic_indices: evaluated.diagnostic_owners,
            effects: evaluated.effects,
            snapshots: Vec::new(),
            owners: Vec::new(),
        });
    }

    let policy = compiler_diagnostics::observed::Policy {
        config: policy,
        metrics: inputs.metrics,
    };
    let mut evaluated = evaluate_core(inputs, config, retention, ctx.args.compiler_patterns)?;
    let compiler_diagnostics::observed::Completed { snapshots, owners } =
        compiler_diagnostics::observed::collect(ctx, unit, &mut evaluated, &policy);
    Ok(Completed {
        evaluation: evaluated.report,
        facts: evaluated.facts,
        diagnostic_indices: evaluated.diagnostic_owners,
        effects: evaluated.effects,
        snapshots,
        owners,
    })
}

fn retention(ctx: &Context<'_>) -> rstyle_core::Retention {
    if ctx.args.review_status_requested()
        || ctx
            .args
            .compiler_baseline_requested(crate::clones::CompilerBaseline::Diagnostics)
    {
        rstyle_core::Retention::Scopes
    } else if ctx.args.review_file.is_some() || ctx.args.compiler_plans {
        rstyle_core::Retention::Owners
    } else {
        rstyle_core::Retention::Report
    }
}

fn evaluate_core(
    inputs: Inputs,
    config: &rstyle_core::config::Config,
    retention: rstyle_core::Retention,
    retain_effects: bool,
) -> Result<rstyle_core::Evaluation, String> {
    let Inputs { facts, effects, .. } = inputs;
    if retain_effects {
        return rstyle_core::evaluate_retaining_effects(facts, config, retention, effects);
    }
    if let Some(effects) = effects {
        return rstyle_core::evaluate_with_effects(facts, config, retention, effects);
    }
    rstyle_core::evaluate_retained(facts, config, retention)
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
