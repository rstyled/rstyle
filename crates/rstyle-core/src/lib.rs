//! Pure evaluation of style rules over facts extracted by `rstyle-driver`.

pub mod align;
pub mod candidates;
pub mod clip;
pub mod cluster;
pub mod config;
pub mod dispatch;
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
pub mod stacking;
pub mod suppress;
pub mod wl;
pub mod wl_trace;

use config::Config;
use facts::FnFacts;
use globset::{Glob, GlobSet, GlobSetBuilder};
use report::{Diagnostic, Report, Summary};
use std::collections::BTreeMap;
use suppress::Suppression;

fn exclude_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| format!("bad exclude `{pattern}`: {e}"))?;
        builder.add(glob);
    }
    builder.build().map_err(|e| e.to_string())
}

fn rule_diagnostics(f: &FnFacts, effects: &[rules::Effect], cfg: &Config) -> Vec<Diagnostic> {
    [
        rules::fn_length(f, cfg),
        rules::fn_params(f, cfg),
        rules::fn_spacing(f, cfg),
        rules::mutation_budget(f, &cfg.mutation_budget),
        rules::side_effect_density(f, effects, cfg),
    ]
    .concat()
}

fn invalid_suppression(f: &FnFacts, message: String) -> Diagnostic {
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

fn function_diagnostics(f: &FnFacts, effects: &[rules::Effect], cfg: &Config) -> Vec<Diagnostic> {
    let mut out = rule_diagnostics(f, effects, cfg);
    match suppress::parse(&f.preceding_line) {
        Suppression::None => {}
        Suppression::Allow(rules) => out.retain(|d| !rules.contains(&d.rule_id)),
        Suppression::Invalid(message) => out.push(invalid_suppression(f, message)),
    }
    out
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
    Summary {
        functions: facts.len(),
        violations,
        mean_density_milli: densities.clone().sum::<usize>() / facts.len().max(1),
        max_density_milli: densities.max().unwrap_or(0),
        effect_counts: effect_counts(effects.iter().flatten()),
    }
}

/// Direct effects of every function plus what it inherits through the call graph.
fn all_effects(facts: &[FnFacts], graph: &graph::Graph, cfg: &Config) -> Vec<Vec<rules::Effect>> {
    let direct: Vec<_> = facts.iter().map(|f| rules::effects_of(f, cfg)).collect();
    let boundary = graph::boundaries(facts);
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
    let excluded = exclude_set(&cfg.exclude.0)?;
    let facts: Vec<FnFacts> = facts::merge(facts)
        .into_iter()
        .filter(|f| !excluded.is_match(&f.path))
        .collect();
    let graph = graph::build(&facts);
    let effects = all_effects(&facts, &graph, cfg);
    let mut diagnostics: Vec<Diagnostic> = facts
        .iter()
        .zip(&effects)
        .flat_map(|(f, e)| function_diagnostics(f, e, cfg))
        .collect();

    report::sort(&mut diagnostics);
    let summary = summarize(&facts, &effects, cfg, diagnostics.len());
    let fix_groups = groups::build(&facts, &graph, &diagnostics, cfg);
    Ok(Report {
        diagnostics,
        summary,
        fix_groups,
    })
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
    fn suppression_removes_only_named_rule() {
        let mut f = four_params("f");
        f.preceding_line = "// rstyle-allow(fn_params): ffi shim".into();
        f.mutations = (1..=4)
            .map(|line| MutationEvent {
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
