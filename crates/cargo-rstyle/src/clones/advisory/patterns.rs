//! Adapt owned compiler graphs to existing core proposal policy.
use super::Missing;
use rstyle_core::{
    advisory::patterns::{KEY, Observation},
    facts::FnFacts,
    patterns_report,
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    policy: rstyle_core::cluster::Params,
    pub input_complete: bool,
    policy_selected_functions: usize,
    observed_graphs: usize,
    unavailable_functions: usize,
    unavailable_graphs: Vec<Missing>,
    pub evaluation: patterns_report::PatternsReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    visible_candidates: Option<Vec<usize>>,
    #[serde(skip)]
    pub snapshots:
        Vec<Result<crate::clones::review::compiler_patterns::observed::Snapshot, String>>,
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
}

pub(super) fn evaluate(
    ctx: &crate::clones::report::Context<'_>,
    unit: &crate::clones::identity::UnitContext,
    input: Inputs,
) -> Report {
    let Inputs {
        facts,
        missing,
        unavailable_functions,
    } = input;

    let options = patterns_report::Options::default();
    let policy_selected_functions = facts.len();
    let observed_graphs = facts
        .iter()
        .filter(|function| function.pdg.is_some())
        .count();

    let (evaluation, prepared) = patterns_report::run_owned_and_retain(facts, &options);
    let snapshots = if ctx.args.review_file.is_some() {
        crate::clones::review::compiler_patterns::observed::collect(
            ctx,
            unit,
            prepared,
            &evaluation.candidates,
        )
    } else {
        Vec::new()
    };
    Report {
        schema: "rstyle.compiler-patterns/1",
        policy: options.params,
        input_complete: missing.is_empty() && unavailable_functions == 0,
        policy_selected_functions,
        observed_graphs,
        unavailable_functions,
        unavailable_graphs: missing,
        evaluation,
        snapshots,
        visible_candidates: None,
    }
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
    pub(super) fn render(&self) -> String {
        patterns_report::text_selected(&self.evaluation, |index| {
            self.visible_candidates
                .as_ref()
                .is_none_or(|visible| visible.contains(&index))
        })
    }
}
