//! Adapt owned compiler graphs to existing core proposal policy.
pub(super) mod workspace;
use super::{Missing, source_scope::SourceScope};
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
    member_units: Vec<crate::clones::identity::UnitId>,
    owners: std::collections::BTreeMap<String, crate::clones::FunctionKey>,
    policy_version: u32,
    policy: rstyle_core::cluster::Params,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    hide_implements: Vec<String>,
    pub input_complete: bool,
    effect_evidence: EffectEvidence,
    policy_selected_functions: usize,
    observed_graphs: usize,
    unavailable_functions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_scope: Option<SourceScope>,
    unavailable_graphs: Vec<Missing>,
    pub evaluation: patterns_report::PatternsReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    presentation: Option<Presentation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visible_candidates: Option<Vec<usize>>,
    /// File selection is independent of review visibility; presentation intersects both masks.
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_candidates: Option<Vec<usize>>,
    #[serde(skip)]
    pub snapshots:
        Vec<Result<crate::clones::review::compiler_patterns::observed::Snapshot, String>>,
}

/// Indices borrow the raw ranked evaluation; review and baseline identity stay unchanged.
#[derive(Serialize)]
struct Presentation {
    min_score_milli: u32,
    top: Option<usize>,
    candidate_indices: Vec<usize>,
    signature_group_indices: Vec<usize>,
}

impl Presentation {
    fn new(args: &crate::PatternDisplayArgs, evaluation: &patterns_report::PatternsReport) -> Self {
        let min_score_milli = args.min_score_milli();
        Self {
            min_score_milli,
            top: args.top,
            candidate_indices: patterns_report::select_scored(
                0..evaluation.candidates.len(),
                |index| evaluation.candidates[*index].score_milli,
                min_score_milli,
                args.top,
            )
            .collect(),
            signature_group_indices: patterns_report::select_scored(
                0..evaluation.signature_groups.len(),
                |index| evaluation.signature_groups[*index].score_milli,
                min_score_milli,
                args.top,
            )
            .collect(),
        }
    }

    fn with_reviews(
        mut self,
        evaluation: &patterns_report::PatternsReport,
        visible: &[usize],
    ) -> Self {
        self.candidate_indices = patterns_report::select_scored(
            visible.iter().copied(),
            |index| evaluation.candidates[*index].score_milli,
            self.min_score_milli,
            self.top,
        )
        .collect();
        self
    }
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
            context: "compiler_advisory units listed by member_units; per-function owners retained",
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
    let Some(raw) = function.facts.view().build.frontend_options.get(KEY) else {
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
    pub generated_functions: Vec<crate::clones::FunctionKey>,
    pub effects: Option<std::collections::BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    pub effects_complete: bool,
}

pub(super) fn evaluate(
    ctx: &crate::clones::report::Context<'_>,
    units: &[&crate::clones::identity::UnitContext],
    input: Inputs,
    scoped: bool,
) -> Result<Report, String> {
    let Inputs {
        facts,
        missing,
        unavailable_functions,
        generated_functions,
        effects,
        effects_complete,
    } = input;

    let source_scope = SourceScope::select(ctx.args.compiler_pattern_scope, generated_functions);
    let unavailable_functions = in_scope_unavailable(unavailable_functions, source_scope.as_ref());

    let options = crate::mine_options(&ctx.args.mine);
    let policy_selected_functions = facts.len();
    let observed_graphs = facts
        .iter()
        .filter(|function| function.pdg.is_some())
        .count();

    let (mut evaluation, prepared) = discover(facts, &options, scoped);
    let selected_candidates = select_candidates(ctx, &evaluation)?;
    let effects = effects.ok_or("Compiler patterns require completed core effects")?;
    rstyle_core::candidates::annotate_completed_effects(&mut evaluation.candidates, &effects)?;
    let owners = owners(ctx, units)?;
    let snapshots = review_snapshots(ctx, units, prepared, &evaluation);
    Ok(Report {
        schema: "rstyle.compiler-patterns/2",
        member_units: units.iter().map(|unit| unit.id).collect(),
        owners,
        policy_version: rstyle_core::candidates::POLICY_VERSION,
        policy: options.params,
        hide_implements: options.hide_implements,
        effect_evidence: EffectEvidence::new(effects_complete),
        input_complete: missing.is_empty() && unavailable_functions == 0,
        policy_selected_functions,
        observed_graphs,
        unavailable_functions,
        source_scope,
        unavailable_graphs: missing,
        presentation: (!ctx.args.pattern_display.is_default())
            .then(|| Presentation::new(&ctx.args.pattern_display, &evaluation)),
        evaluation,
        snapshots,
        visible_candidates: None,
        selected_candidates,
    })
}

fn discover(
    facts: Vec<FnFacts>,
    options: &patterns_report::Options,
    scoped: bool,
) -> (patterns_report::PatternsReport, Vec<FnFacts>) {
    let (evaluation, prepared) = patterns_report::run_distinct_owned_and_retain(facts, options);
    if scoped {
        workspace::restore_evaluation(evaluation, prepared)
    } else {
        (evaluation, prepared)
    }
}

fn in_scope_unavailable(count: usize, scope: Option<&SourceScope>) -> usize {
    count - scope.map_or(0, |scope| scope.outside_scope_functions.len())
}

impl Inputs {
    fn append(self, input: Self) -> Result<Self, String> {
        let mut effects = self.effects.ok_or("Missing completed compiler effects")?;
        effects.extend(input.effects.ok_or("Missing completed compiler effects")?);

        Ok(Self {
            facts: joined(self.facts, input.facts),
            missing: joined(self.missing, input.missing),
            generated_functions: joined(self.generated_functions, input.generated_functions),
            unavailable_functions: self.unavailable_functions + input.unavailable_functions,
            effects_complete: self.effects_complete && input.effects_complete,
            effects: Some(effects),
        })
    }
}

fn joined<T>(mut values: Vec<T>, additional: Vec<T>) -> Vec<T> {
    values.extend(additional);
    values
}

fn owners(
    ctx: &crate::clones::report::Context<'_>,
    units: &[&crate::clones::identity::UnitContext],
) -> Result<std::collections::BTreeMap<String, crate::clones::FunctionKey>, String> {
    let mut owners = std::collections::BTreeMap::new();
    for unit in units {
        for (id, record) in crate::clones::review::compiler_owners::originals(ctx, unit.id) {
            if owners.insert(id.to_owned(), record.key).is_some() {
                return Err("Compiler pattern population has duplicate original owners".into());
            }
        }
    }
    Ok(owners)
}

fn review_snapshots(
    ctx: &crate::clones::report::Context<'_>,
    units: &[&crate::clones::identity::UnitContext],
    prepared: Vec<FnFacts>,
    evaluation: &patterns_report::PatternsReport,
) -> Vec<Result<observed::Snapshot, String>> {
    if ctx.args.review_file.is_some()
        || ctx.args.legacy_pattern_ledger.is_some()
        || ctx
            .args
            .compiler_baseline_requested(crate::clones::CompilerBaseline::Patterns)
    {
        observed::collect(ctx, units, prepared, &evaluation.candidates)
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
    pub(super) fn missing_graphs(&self) -> impl Iterator<Item = crate::clones::FunctionKey> + '_ {
        self.unavailable_graphs.iter().map(|missing| missing.key)
    }

    pub(super) fn outside_scope(&self) -> impl Iterator<Item = crate::clones::FunctionKey> + '_ {
        self.source_scope
            .iter()
            .flat_map(|scope| scope.outside_scope_functions.iter().copied())
    }

    pub(super) fn with_reviews(
        self,
        unit: crate::clones::identity::UnitId,
        reviews: &crate::clones::review::compiler_patterns::Report,
    ) -> Self {
        let visible_candidates: Vec<_> = (0..self.evaluation.candidates.len())
            .filter(|index| !reviews.hidden_for(unit, *index))
            .collect();
        let presentation = self
            .presentation
            .map(|presentation| presentation.with_reviews(&self.evaluation, &visible_candidates));
        Self {
            visible_candidates: Some(visible_candidates),
            presentation,
            ..self
        }
    }
    pub(super) fn findings(&self) -> usize {
        self.shown_candidates().count()
    }

    fn shown_candidates(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.evaluation.candidates.len()).filter(|index| self.shown(*index))
    }
    pub(super) fn selection(&self) -> Option<&[usize]> {
        self.selected_candidates.as_deref()
    }

    fn visible(&self, index: usize) -> bool {
        self.visible_candidates
            .as_ref()
            .is_none_or(|visible| visible.binary_search(&index).is_ok())
    }

    fn shown(&self, index: usize) -> bool {
        self.visible(index)
            && self.presentation.as_ref().is_none_or(|presentation| {
                presentation.candidate_indices.binary_search(&index).is_ok()
            })
    }

    fn selected(&self, index: usize) -> bool {
        self.selection()
            .is_none_or(|selected| selected.binary_search(&index).is_ok())
    }

    pub(super) fn selected_findings(&self) -> usize {
        self.shown_candidates()
            .filter(|index| self.selected(*index))
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
            .filter(|(index, _)| self.shown(*index) && self.selected(*index))
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
        let evaluation = patterns_report::text_selected_with_groups(
            &self.evaluation,
            |index| self.shown(index) && self.selected(index),
            |index| {
                self.presentation.as_ref().is_none_or(|presentation| {
                    presentation
                        .signature_group_indices
                        .binary_search(&index)
                        .is_ok()
                })
            },
        );
        let Some(scope) = &self.source_scope else {
            return evaluation;
        };
        format!("{}{evaluation}", scope.render(self.unavailable_functions))
    }
}
