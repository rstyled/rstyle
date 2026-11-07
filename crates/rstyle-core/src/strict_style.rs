//! Opinions about how code could be shaped rather than limits it passed, so each is off unless
//! `[strict_style]` turns it on. One that's on reports through `check`, and a run fails on it, as
//! on any other style rule.

mod primitive_obsession;
mod value_object;

use crate::config::Config;
use crate::facts::FnFacts;
use crate::report::{Detail, Diagnostic};
use crate::rules::{Measure, diagnostic};

pub fn guard_clause(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let rule = &cfg.strict_style.guard_clause;
    let lines: Vec<usize> = f
        .guard_clauses
        .iter()
        .filter(|guard| rule.enabled && guard.statements >= rule.min_nested_statements)
        .map(|guard| guard.line)
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let actual = lines.len();
    let written: Vec<String> = lines.iter().map(ToString::to_string).collect();
    let message = format!(
        "function `{}` nests its main path under an `if` whose `else` only returns, on line {}; \
         return early instead",
        f.name,
        written.join(", "),
    );

    let mut found = diagnostic(f, "guard_clause", message, Measure { actual, limit: 0 });
    found.details = vec![Detail {
        label: "if".into(),
        count: actual,
        lines,
    }];
    vec![found]
}

/// What the rules only a whole run can judge find, one list per function, in the order of `facts`.
pub fn across_the_run(facts: &[FnFacts], cfg: &Config) -> Vec<Vec<Diagnostic>> {
    let mut found = vec![Vec::new(); facts.len()];
    let rules = &cfg.strict_style;
    if rules.primitive_obsession.enabled {
        primitive_obsession::add(facts, &rules.primitive_obsession, &mut found);
    }
    if rules.value_object {
        value_object::add(facts, &mut found);
    }
    found
}

#[cfg(test)]
mod tests;
