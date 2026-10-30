//! Retain completed core owners only for explicit review requests.
use crate::clones::review::compiler_diagnostics::demand::Demand;
use crate::clones::{
    identity::{UnitContext, UnitId},
    report::Context,
    review::compiler_diagnostics,
};
use rstyle_core::{facts::FnFacts, report::Report};

#[derive(Default)]
pub(super) struct Completed {
    pub evaluation: Option<Report>,
    pub facts: Vec<FnFacts>,
    pub diagnostic_indices: Vec<usize>,
    pub effects: Option<std::collections::BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    pub snapshots: Vec<Result<compiler_diagnostics::Snapshot, String>>,
    pub owners: Vec<compiler_diagnostics::observed::Assessed>,
    pub owner_names: std::collections::BTreeMap<String, usize>,
    pub owner_scope_truncated: bool,
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
    demand: &Demand<'_>,
) -> Result<Completed, String> {
    let config = ctx
        .compiler_advisory_config
        .ok_or("Compiler input policy was not requested")?;
    if !ctx.args.compiler_effects_requested() {
        return Ok(Completed {
            facts: rstyle_core::selected_observations(inputs.facts, config)?,
            ..Completed::default()
        });
    }
    if !ctx.args.compiler_advisory {
        return patterns_only(inputs, config);
    }
    let retention = retention(ctx);
    if ctx.args.review_file.is_none()
        && !ctx
            .args
            .compiler_baseline_requested(crate::clones::CompilerBaseline::Diagnostics)
    {
        return evaluate_core(inputs, config, retention, ctx.args.compiler_patterns)
            .map(Completed::from_evaluation);
    }

    let policy = compiler_diagnostics::observed::Policy {
        config: &demand.policy,
        metrics: inputs.metrics,
    };
    let mut evaluated = evaluate_core(inputs, config, retention, ctx.args.compiler_patterns)?;
    let observed = compiler_diagnostics::observed::collect(
        ctx,
        unit,
        &mut evaluated,
        compiler_diagnostics::observed::Request {
            policy,
            demand,
            rules: ctx.args.compiler_diagnostic_request(),
        },
    );
    Ok(Completed {
        snapshots: observed.snapshots,
        owners: observed.owners,
        owner_names: observed.owner_names,
        owner_scope_truncated: observed.owner_scope_truncated,
        ..Completed::from_evaluation(evaluated)
    })
}

impl Completed {
    fn from_evaluation(evaluated: rstyle_core::Evaluation) -> Self {
        Self {
            evaluation: Some(evaluated.report),
            facts: evaluated.facts,
            diagnostic_indices: evaluated.diagnostic_owners,
            effects: evaluated.effects,
            ..Self::default()
        }
    }
}

fn patterns_only(
    inputs: Inputs,
    config: &rstyle_core::config::Config,
) -> Result<Completed, String> {
    let prepared = rstyle_core::effect_observations(inputs.facts, config, inputs.effects)?;
    Ok(Completed {
        evaluation: None,
        facts: prepared.facts,
        diagnostic_indices: Vec::new(),
        effects: Some(prepared.effects),
        snapshots: Vec::new(),
        owners: Vec::new(),
        owner_names: std::collections::BTreeMap::new(),
        owner_scope_truncated: false,
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
