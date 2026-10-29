//! Pure evaluation of style rules over facts extracted by `rstyle-driver`.

pub mod advisory;
pub mod align;
pub mod candidates;
pub mod cli_args;
pub mod clip;
pub mod cluster;
pub mod complexity;
pub mod complexity_report;
pub mod config;
pub mod dispatch;
pub mod exclusions;
pub mod facts;
pub mod graph;
pub mod groups;
pub mod ledger;
pub mod lines;
pub mod locator;
pub mod normalize;
pub mod paths;
pub mod patterns_report;
#[cfg(test)]
mod pdgtest;
pub mod prep;
pub mod report;
pub mod rules;
pub mod shape;
pub mod sigmine;
pub mod spacing;
pub mod stacking;
pub mod suppress;
pub mod wl;
pub mod wl_trace;

use config::Config;
use facts::FnFacts;
use report::{Diagnostic, Report, Summary};
use std::collections::BTreeMap;
use suppress::Suppression;

fn rule_diagnostics(f: &FnFacts, effects: &[rules::Effect], cfg: &Config) -> Vec<Diagnostic> {
    [
        rules::fn_length(f, cfg),
        rules::fn_params(f, cfg),
        rules::fn_spacing(f, cfg),
        rules::complexity(f, cfg),
        rules::mutation_budget(f, &cfg.mutation_budget),
        rules::side_effect_density(f, effects, cfg),
    ]
    .concat()
}

fn invalid_suppression(f: &FnFacts, message: &str) -> Diagnostic {
    Diagnostic {
        path: f.path.clone(),
        line: f.line,
        column: f.column,
        rule_id: "invalid_suppression".into(),
        message: format!("function `{}`: {message}", f.name),
        actual: 1,
        limit: 0,
        details: vec![],
    }
}

fn function_diagnostics(
    f: &FnFacts,
    effects: &[rules::Effect],
    cfg: &Config,
) -> (Vec<Diagnostic>, Suppression) {
    let mut out = rule_diagnostics(f, effects, cfg);
    let suppression = suppress::parse(&f.preceding_line);
    match &suppression {
        Suppression::None => {}
        Suppression::Allow(rules) => out.retain(|d| !rules.contains(&d.rule_id)),
        Suppression::Invalid(message) => out.push(invalid_suppression(f, message)),
    }
    (out, suppression)
}

fn effect_counts<'a>(effects: impl Iterator<Item = &'a rules::Effect>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for effect in effects {
        *counts.entry(effect.kind.name().to_string()).or_default() += 1;
    }
    counts
}

fn summarize(
    facts: &[FnFacts],
    effects: &[Vec<rules::Effect>],
    cfg: &Config,
    violations: usize,
) -> Summary {
    let densities: Vec<usize> = facts
        .iter()
        .zip(effects)
        .map(|(f, e)| rules::density_milli(f, e, cfg))
        .collect();
    let densities = densities.into_iter();
    let measured = facts
        .iter()
        .filter(|fact| fact.complexity.is_some())
        .count();
    Summary {
        functions: facts.len(),
        violations,
        complexity_measured: measured,
        complexity_unavailable: facts.len() - measured,
        mean_density_milli: densities.clone().sum::<usize>() / facts.len().max(1),
        max_density_milli: densities.max().unwrap_or(0),
        effect_counts: effect_counts(effects.iter().flatten()),
    }
}

/// Direct effects of every function plus what it inherits through the call graph.
pub fn all_effects<F: std::borrow::Borrow<FnFacts>>(
    facts: &[F],
    graph: &graph::Graph,
    cfg: &Config,
) -> Vec<Vec<rules::Effect>> {
    let direct: Vec<_> = facts
        .iter()
        .map(|f| rules::effects_of(f.borrow(), cfg))
        .collect();
    let boundary: Vec<_> = facts
        .iter()
        .map(|facts| graph::is_boundary(facts.borrow()))
        .collect();
    let inputs = graph::Inputs {
        facts,
        graph,
        direct: &direct,
        boundary: &boundary,
    };
    graph::propagate(&inputs, cfg.side_effects.transitive_depth)
}

/// Evaluates all rules. Input order does not matter: facts are merged first.
pub fn evaluate(facts: Vec<FnFacts>, cfg: &Config) -> Result<Report, String> {
    evaluate_and_retain(facts, cfg).map(|(report, _)| report)
}

/// Return the merged, path-selected observations for another consumer without copying bodies.
pub fn evaluate_and_retain(
    facts: Vec<FnFacts>,
    cfg: &Config,
) -> Result<(Report, Vec<FnFacts>), String> {
    evaluate_inputs(facts, cfg, Retention::Report, None)
        .map(|evaluation| (evaluation.report, evaluation.facts))
}

/// Completed diagnostic ownership follows report sorting without location-based inference.
pub struct Evaluation {
    pub report: Report,
    pub facts: Vec<FnFacts>,
    /// One index into `facts` for each diagnostic in `report`.
    pub diagnostic_owners: Vec<usize>,
    /// Explicit consumers may retain completed suppression scopes and excluded observations.
    pub scopes: Option<Scopes>,
}

#[derive(Default)]
pub struct Scopes {
    /// Aligned with the selected facts, from their completed rule evaluation.
    pub suppressions: Vec<Suppression>,
    pub excluded: Vec<FnFacts>,
}

#[derive(Clone, Copy)]
pub enum Retention {
    Report,
    Owners,
    Scopes,
}

pub fn evaluate_with_owners(facts: Vec<FnFacts>, cfg: &Config) -> Result<Evaluation, String> {
    evaluate_inputs(facts, cfg, Retention::Owners, None)
}

/// Retain completed numeric scope outcomes without evaluating a rule or parsing a comment again.
pub fn evaluate_with_scopes(facts: Vec<FnFacts>, cfg: &Config) -> Result<Evaluation, String> {
    evaluate_inputs(facts, cfg, Retention::Scopes, None)
}

/// Evaluate with only the requested run-local metadata retained.
pub fn evaluate_retained(
    facts: Vec<FnFacts>,
    cfg: &Config,
    retention: Retention,
) -> Result<Evaluation, String> {
    evaluate_inputs(facts, cfg, retention, None)
}

/// Consume completed effects from the same policy and scoped owners; missing results are errors.
/// The consumer resolves cross-unit targets; core still owns propagation and rule policy.
pub fn evaluate_with_effects(
    facts: Vec<FnFacts>,
    cfg: &Config,
    retention: Retention,
    effects: BTreeMap<String, Vec<rules::Effect>>,
) -> Result<Evaluation, String> {
    evaluate_inputs(facts, cfg, retention, Some(effects))
}

struct RuleInputs<'a> {
    facts: &'a [FnFacts],
    effects: &'a [Vec<rules::Effect>],
    cfg: &'a Config,
}

fn evaluated_diagnostics(
    inputs: RuleInputs<'_>,
    retention: Retention,
    scopes: &mut Option<Scopes>,
) -> (Vec<usize>, Vec<Diagnostic>) {
    if matches!(retention, Retention::Report) {
        let mut diagnostics = collect_diagnostics(inputs, |_, diagnostic| diagnostic, None);
        report::sort(&mut diagnostics);
        return (Vec::new(), diagnostics);
    }
    let mut owned = collect_diagnostics(
        inputs,
        |owner, diagnostic| (owner, diagnostic),
        scopes.as_mut().map(|scopes| &mut scopes.suppressions),
    );
    owned.sort_by(|(_, a), (_, b)| report::ordering(a, b));
    owned.into_iter().unzip()
}

fn collect_diagnostics<T>(
    inputs: RuleInputs<'_>,
    emit: impl Fn(usize, Diagnostic) -> T,
    mut scopes: Option<&mut Vec<Suppression>>,
) -> Vec<T> {
    inputs
        .facts
        .iter()
        .zip(inputs.effects)
        .enumerate()
        .flat_map(|(owner, (f, e))| {
            let emit = &emit;
            let (diagnostics, suppression) = function_diagnostics(f, e, inputs.cfg);
            if let Some(scopes) = scopes.as_mut() {
                scopes.push(suppression);
            }
            diagnostics
                .into_iter()
                .map(move |diagnostic| emit(owner, diagnostic))
        })
        .collect()
}

fn evaluate_inputs(
    facts: Vec<FnFacts>,
    cfg: &Config,
    retention: Retention,
    completed: Option<BTreeMap<String, Vec<rules::Effect>>>,
) -> Result<Evaluation, String> {
    let excluded = exclusions::Matcher::compile(&cfg.exclude.0)?;
    let mut scopes = matches!(retention, Retention::Scopes).then(Scopes::default);
    let facts: Vec<FnFacts> = facts::merge(facts)
        .into_iter()
        .filter_map(|fact| {
            if excluded.is_excluded(std::path::Path::new(&fact.path)) {
                if let Some(scopes) = scopes.as_mut() {
                    scopes.excluded.push(fact);
                }
                None
            } else {
                Some(fact)
            }
        })
        .collect();
    let graph = graph::build(&facts);
    let effects = completed_effects(&facts, &graph, cfg, completed)?;

    let (diagnostic_owners, diagnostics) = evaluated_diagnostics(
        RuleInputs {
            facts: &facts,
            effects: &effects,
            cfg,
        },
        retention,
        &mut scopes,
    );
    let summary = summarize(&facts, &effects, cfg, diagnostics.len());
    let fix_groups = groups::build(&facts, &graph, &diagnostics, cfg);
    let functions = complexity_report::findings(&facts, &diagnostics);
    Ok(Evaluation {
        report: Report {
            diagnostics,
            summary,
            fix_groups,
            functions,
        },
        facts,
        diagnostic_owners,
        scopes,
    })
}

fn completed_effects(
    facts: &[FnFacts],
    graph: &graph::Graph,
    cfg: &Config,
    completed: Option<BTreeMap<String, Vec<rules::Effect>>>,
) -> Result<Vec<Vec<rules::Effect>>, String> {
    match completed {
        None => Ok(all_effects(facts, graph, cfg)),
        Some(mut completed) => facts
            .iter()
            .map(|fact| {
                completed
                    .remove(&fact.id)
                    .ok_or_else(|| "Missing completed compiler effects".to_string())
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use facts::{MutationEvent, sample};

    fn four_params(name: &str) -> FnFacts {
        FnFacts {
            params: 5,
            ..sample(name)
        }
    }

    #[test]
    fn default_evaluation_keeps_no_owner_roster_and_preserves_the_report() {
        let plain = evaluate_inputs(
            vec![four_params("original")],
            &Config::default(),
            Retention::Report,
            None,
        )
        .unwrap();
        let owned =
            evaluate_with_owners(vec![four_params("original")], &Config::default()).unwrap();
        assert!(plain.scopes.is_none());
        assert!(owned.scopes.is_none());
        assert_eq!(plain.diagnostic_owners.capacity(), 0);
        assert_eq!(plain.report, owned.report);
        assert_eq!(owned.diagnostic_owners, vec![0]);
    }

    #[test]
    fn borrowed_completed_effects_preserve_reports_and_require_every_selected_owner() {
        let facts = four_params("owner");
        let buffer = facts.source.as_ptr();
        let config = Config::default();
        let borrowed = vec![&facts];
        let graph = graph::build_resolved(&borrowed, |_, _| Vec::new());
        let effects = all_effects(&borrowed, &graph, &config);
        let completed = BTreeMap::from([(facts.id.clone(), effects.into_iter().next().unwrap())]);
        let expected = evaluate(vec![facts.clone()], &config).unwrap();
        let evaluated =
            evaluate_with_effects(vec![facts], &config, Retention::Scopes, completed).unwrap();
        assert_eq!(evaluated.report, expected);
        assert_eq!(evaluated.facts[0].source.as_ptr(), buffer);
        assert!(
            evaluate_with_effects(
                vec![four_params("missing")],
                &config,
                Retention::Report,
                BTreeMap::new()
            )
            .is_err()
        );
    }

    #[test]
    fn completed_scopes_retain_suppressions_and_move_excluded_source_buffers() {
        let mut excluded = four_params("excluded");
        excluded.path = "generated/file.rs".into();
        let buffer = excluded.source.as_ptr();
        let mut allowed = four_params("allowed");
        allowed.preceding_line = "// rstyle-allow(fn_params): fixture".into();
        let config: Config = toml::from_str("exclude=['generated/**']").unwrap();
        let original = vec![excluded, allowed, four_params("reported")];
        let expected = evaluate(original.clone(), &config).unwrap();
        let evaluated = evaluate_with_scopes(original, &config).unwrap();
        assert_eq!(evaluated.report, expected);
        let scopes = evaluated.scopes.unwrap();
        assert_eq!(scopes.excluded.len(), 1);
        assert_eq!(scopes.excluded[0].source.as_ptr(), buffer);
        assert_eq!(scopes.suppressions.len(), evaluated.facts.len());
        for (fact, suppression) in evaluated.facts.iter().zip(&scopes.suppressions) {
            match fact.name.as_str() {
                "allowed" => assert_eq!(suppression, &Suppression::Allow(vec!["fn_params".into()])),
                "reported" => assert_eq!(suppression, &Suppression::None),
                other => panic!("unexpected selected owner: {other}"),
            }
        }
        assert_eq!(evaluated.report.diagnostics.len(), 1);
        assert_eq!(
            evaluated.facts[evaluated.diagnostic_owners[0]].name,
            "reported"
        );
    }

    #[test]
    fn completed_diagnostic_owners_survive_sorting_and_shared_source_positions() {
        let mut first = four_params("zeta");
        first.path = "same.rs".into();
        first.line = 3;
        first.column = 8;
        let mut second = four_params("alpha");
        second.path = "same.rs".into();
        second.line = 3;
        second.column = 2;
        let buffers = [first.source.as_ptr(), second.source.as_ptr()];
        let evaluated = evaluate_with_owners(vec![first, second], &Config::default()).unwrap();
        let associations: Vec<_> = evaluated
            .report
            .diagnostics
            .iter()
            .zip(&evaluated.diagnostic_owners)
            .map(|(diagnostic, owner)| {
                (
                    diagnostic.column,
                    diagnostic.rule_id.as_str(),
                    evaluated.facts[*owner].name.as_str(),
                )
            })
            .collect();
        assert_eq!(
            associations,
            vec![(2, "fn_params", "alpha"), (8, "fn_params", "zeta")]
        );
        assert!(
            evaluated
                .facts
                .iter()
                .all(|fact| buffers.contains(&fact.source.as_ptr()))
        );
    }

    #[test]
    fn suppression_removes_only_named_rule() {
        let mut f = four_params("f");
        f.preceding_line = "// rstyle-allow(fn_params): ffi shim".into();
        f.mutations = (1..=4)
            .map(|line| MutationEvent {
                origin: crate::facts::MutationOrigin::Source,
                root: "v".into(),
                field_path: String::new(),
                line,
                escapes: true,
                delegated: false,
            })
            .collect();
        let report = evaluate(vec![f], &Config::default()).unwrap();
        let ids: Vec<_> = report
            .diagnostics
            .iter()
            .map(|d| d.rule_id.as_str())
            .collect();
        assert_eq!(ids, ["mutation_per_target"]);
    }

    #[test]
    fn suppression_without_reason_is_reported() {
        let mut f = four_params("f");
        f.preceding_line = "// rstyle-allow(fn_params)".into();
        let report = evaluate(vec![f], &Config::default()).unwrap();
        let ids: Vec<_> = report
            .diagnostics
            .iter()
            .map(|d| d.rule_id.as_str())
            .collect();
        assert_eq!(ids, ["fn_params", "invalid_suppression"]);
    }

    #[test]
    fn output_is_independent_of_input_order() {
        let mut b = four_params("b");
        b.path = "src/b.rs".into();
        let one = evaluate(vec![four_params("a"), b.clone()], &Config::default()).unwrap();
        let two = evaluate(vec![b, four_params("a")], &Config::default()).unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn excluded_paths_are_dropped() {
        let mut f = four_params("f");
        f.path = "target/gen/x.rs".into();
        let report = evaluate(vec![f], &Config::default()).unwrap();
        assert_eq!(report.summary.functions, 0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn retained_inputs_follow_policy_exclusions_without_copying_body_buffers() {
        let mut kept = four_params("kept");
        kept.pdg = Some(crate::pdgtest::emit_loop("Cow", "value"));
        let source = kept.source.as_ptr();
        let nodes = kept.pdg.as_ref().unwrap().nodes.as_ptr();
        let mut excluded = sample("excluded");
        excluded.path = "target/generated.rs".into();
        let (report, retained) =
            evaluate_and_retain(vec![excluded, kept], &Config::default()).unwrap();
        assert_eq!(report.summary.functions, 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|row| row.rule_id == "fn_params")
        );
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].source.as_ptr(), source);
        assert_eq!(retained[0].pdg.as_ref().unwrap().nodes.as_ptr(), nodes);
    }

    #[test]
    fn summary_math() {
        let mut heavy = sample("heavy");
        heavy.line = 5;
        heavy.statements = 4;
        heavy.calls = vec![facts::CallEvent {
            callee: "std::fs::write".into(),
            line: 6,
            dynamic: false,
        }];
        let report = evaluate(vec![sample("pure"), heavy], &Config::default()).unwrap();
        assert_eq!(report.summary.functions, 2);
        assert_eq!(report.summary.max_density_milli, 750);
        assert_eq!(report.summary.mean_density_milli, 375);
        assert_eq!(report.summary.effect_counts["io"], 1);
    }
}
