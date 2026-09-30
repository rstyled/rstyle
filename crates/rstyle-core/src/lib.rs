//! Pure evaluation of style rules over facts extracted by `rstyle-driver`.

pub mod config;
pub mod facts;
pub mod lines;
pub mod paths;
pub mod report;
pub mod rules;
pub mod suppress;

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

fn rule_diagnostics(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let mut out = rules::fn_length(f, cfg);
    out.extend(rules::fn_params(f, cfg));
    out.extend(rules::mutation_budget(f, &cfg.mutation_budget));
    out.extend(rules::side_effect_density(f, cfg));
    out
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

fn function_diagnostics(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let mut out = rule_diagnostics(f, cfg);
    match suppress::parse(&f.preceding_line) {
        Suppression::None => {}
        Suppression::Allow(rules) => out.retain(|d| !rules.contains(&d.rule_id)),
        Suppression::Invalid(message) => out.push(invalid_suppression(f, message)),
    }
    out
}

fn score(f: &FnFacts, cfg: &Config) -> (usize, Vec<rules::Effect>) {
    let effects = rules::effects_of(f, cfg);
    (rules::density_milli(f, &effects, cfg), effects)
}

fn effect_counts<'a>(effects: impl Iterator<Item = &'a rules::Effect>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for effect in effects {
        *counts.entry(effect.kind.name().to_string()).or_default() += 1;
    }
    counts
}

fn summarize(facts: &[FnFacts], cfg: &Config, violations: usize) -> Summary {
    let scored: Vec<_> = facts.iter().map(|f| score(f, cfg)).collect();
    let densities = scored.iter().map(|(density, _)| *density);
    Summary {
        functions: facts.len(),
        violations,
        mean_density_milli: densities.clone().sum::<usize>() / facts.len().max(1),
        max_density_milli: densities.max().unwrap_or(0),
        effect_counts: effect_counts(scored.iter().flat_map(|(_, effects)| effects)),
    }
}

/// Evaluates all rules. Input order does not matter: facts are merged first.
pub fn evaluate(facts: Vec<FnFacts>, cfg: &Config) -> Result<Report, String> {
    let excluded = exclude_set(&cfg.exclude.0)?;
    let facts: Vec<FnFacts> = facts::merge(facts)
        .into_iter()
        .filter(|f| !excluded.is_match(&f.path))
        .collect();
    let mut diagnostics: Vec<Diagnostic> = facts
        .iter()
        .flat_map(|f| function_diagnostics(f, cfg))
        .collect();
    report::sort(&mut diagnostics);
    let summary = summarize(&facts, cfg, diagnostics.len());
    Ok(Report {
        diagnostics,
        summary,
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
