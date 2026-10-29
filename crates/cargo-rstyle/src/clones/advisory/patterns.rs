//! Adapt owned compiler graphs to existing core proposal policy.
use super::Missing;
use crate::clones::review::compiler_patterns::observed;
use rstyle_core::{
    advisory::patterns::{KEY, Observation},
    facts::FnFacts,
    patterns_report,
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    policy_version: u32,
    policy: rstyle_core::cluster::Params,
    pub input_complete: bool,
    effect_evidence: EffectEvidence,
    policy_selected_functions: usize,
    observed_graphs: usize,
    unavailable_functions: usize,
    unavailable_graphs: Vec<Missing>,
    pub evaluation: patterns_report::PatternsReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    visible_candidates: Option<Vec<usize>>,
    /// File selection is independent of review visibility; presentation intersects both masks.
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_candidates: Option<Vec<usize>>,
    #[serde(skip)]
    pub snapshots:
        Vec<Result<crate::clones::review::compiler_patterns::observed::Snapshot, String>>,
}

/// References to the containing report avoid copying configuration or build context.
/// These annotations are recomputed; structural reviews do not approve effect policy.
#[derive(Serialize)]
struct EffectEvidence {
    schema: &'static str,
    basis: &'static str,
    policy: &'static str,
    metrics: &'static str,
    context: &'static str,
    input_complete: bool,
    review_identity: &'static str,
}

impl EffectEvidence {
    fn new(input_complete: bool) -> Self {
        Self {
            schema: "rstyle.compiler-pattern-effects/1",
            basis: "completed_core_policy_effect_kinds",
            policy: "compiler_advisory.effective_config",
            metrics: "compiler_advisory.metrics",
            context: "containing compiler_advisory unit",
            input_complete,
            review_identity: "structural; effect annotations recomputed under current policy",
        }
    }
}

pub(super) fn attach(
    function: &crate::clones::Function,
    facts: &mut FnFacts,
    missing: &mut Vec<Missing>,
) -> Result<(), String> {
    let Some(raw) = function.facts.build.frontend_options.get(KEY) else {
        missing.push(Missing {
            key: function.key,
            reason: "Compiler pattern graph was not captured".into(),
        });
        return Ok(());
    };
    let observation = Observation::read(raw)?;
    if observation.owner != facts.id {
        return Err("Compiler pattern graph belongs to another function".into());
    }

    facts.pdg = Some(observation.graph);
    Ok(())
}

pub(super) struct Inputs {
    pub facts: Vec<FnFacts>,
    pub missing: Vec<Missing>,
    pub unavailable_functions: usize,
    pub effects: Option<std::collections::BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    pub effects_complete: bool,
}

pub(super) fn evaluate(
    ctx: &crate::clones::report::Context<'_>,
    unit: &crate::clones::identity::UnitContext,
    input: Inputs,
) -> Result<Report, String> {
    let Inputs {
        facts,
        missing,
        unavailable_functions,
        effects,
        effects_complete,
    } = input;

    let options = patterns_report::Options::default();
    let policy_selected_functions = facts.len();
    let observed_graphs = facts
        .iter()
        .filter(|function| function.pdg.is_some())
        .count();

    let (mut evaluation, prepared) = patterns_report::run_owned_and_retain(facts, &options);
    let selected_candidates = select_candidates(ctx, &evaluation)?;
    let effects = effects.ok_or("Compiler patterns require completed core effects")?;
    rstyle_core::candidates::annotate_completed_effects(&mut evaluation.candidates, &effects)?;
    let snapshots = review_snapshots(ctx, unit, prepared, &evaluation);
    Ok(Report {
        schema: "rstyle.compiler-patterns/1",
        policy_version: rstyle_core::candidates::POLICY_VERSION,
        policy: options.params,
        effect_evidence: EffectEvidence::new(effects_complete),
        input_complete: missing.is_empty() && unavailable_functions == 0,
        policy_selected_functions,
        observed_graphs,
        unavailable_functions,
        unavailable_graphs: missing,
        evaluation,
        snapshots,
        visible_candidates: None,
        selected_candidates,
    })
}

fn review_snapshots(
    ctx: &crate::clones::report::Context<'_>,
    unit: &crate::clones::identity::UnitContext,
    prepared: Vec<FnFacts>,
    evaluation: &patterns_report::PatternsReport,
) -> Vec<Result<observed::Snapshot, String>> {
    if ctx.args.review_file.is_some()
        || ctx
            .args
            .compiler_baseline_requested(crate::clones::CompilerBaseline::Patterns)
    {
        observed::collect(ctx, unit, prepared, &evaluation.candidates)
    } else {
        Vec::new()
    }
}

fn select_candidates(
    ctx: &crate::clones::report::Context<'_>,
    evaluation: &patterns_report::PatternsReport,
) -> Result<Option<Vec<usize>>, String> {
    if !ctx.args.select_compiler_patterns {
        return Ok(None);
    }
    let selection = ctx
        .analysis_config
        .and_then(crate::clones::analysis::Config::changed_files)
        .ok_or("Compiler proposal selection needs changed files")?;
    Ok(Some(
        evaluation
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                candidate
                    .sites
                    .iter()
                    .chain(&candidate.definitions)
                    .any(|site| selection.contains(&site.path))
            })
            .map(|(index, _)| index)
            .collect(),
    ))
}

impl Report {
    pub(super) fn with_reviews(
        self,
        unit: crate::clones::identity::UnitId,
        reviews: &crate::clones::review::compiler_patterns::Report,
    ) -> Self {
        let visible_candidates = Some(
            (0..self.evaluation.candidates.len())
                .filter(|index| !reviews.hidden_for(unit, *index))
                .collect(),
        );
        Self {
            visible_candidates,
            ..self
        }
    }
    pub(super) fn findings(&self) -> usize {
        self.visible_candidates
            .as_ref()
            .map_or(self.evaluation.candidates.len(), Vec::len)
    }
    pub(super) fn selection(&self) -> Option<&[usize]> {
        self.selected_candidates.as_deref()
    }

    fn visible(&self, index: usize) -> bool {
        self.visible_candidates
            .as_ref()
            .is_none_or(|visible| visible.binary_search(&index).is_ok())
    }

    fn selected(&self, index: usize) -> bool {
        self.selection()
            .is_none_or(|selected| selected.binary_search(&index).is_ok())
    }

    pub(super) fn selected_findings(&self) -> usize {
        (0..self.evaluation.candidates.len())
            .filter(|index| self.visible(*index) && self.selected(*index))
            .count()
    }

    /// Count a selected proposal once, at its first selected usage or definition path.
    pub(super) fn selected_paths<'a>(
        &'a self,
        selection: &'a crate::clones::analysis::changed_scope::Selection,
    ) -> impl Iterator<Item = &'a str> {
        self.evaluation
            .candidates
            .iter()
            .enumerate()
            .filter(|(index, _)| self.visible(*index) && self.selected(*index))
            .filter_map(|(_, candidate)| {
                candidate
                    .sites
                    .iter()
                    .chain(&candidate.definitions)
                    .find(|site| selection.contains(&site.path))
                    .map(|site| site.path.as_str())
            })
    }

    pub(super) fn render(&self) -> String {
        patterns_report::text_selected(&self.evaluation, |index| {
            self.visible(index) && self.selected(index)
        })
    }
}
