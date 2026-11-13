//! Opinions about how code could be shaped rather than limits it passed, so each is off unless
//! `[strict_style]` turns it on. One that's on reports through `check`, and a run fails on it, as
//! on any other style rule.

mod primitive_obsession;
mod value_object;

use crate::config::Config;
use crate::facts::{CommentKind, FnFacts};
use crate::report::{Detail, Diagnostic};
use crate::rules::{Measure, diagnostic};

/// Flags fns with `min_statements`+ top-level statements and no blank line anywhere.
/// Statements, not lines: a long initializer is still one thing to read. Reported as
/// `actual > limit` with `limit = min_statements - 1`, like the other threshold rules.
pub fn fn_spacing(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let rule = &cfg.strict_style.fn_spacing;
    let min = rule.min_statements;
    let actual = f.top_statements;
    let limit = min.saturating_sub(1);
    let message = format!("function `{}` has no blank line between its phases", f.name);
    let flagged = rule.enabled && crate::spacing::is_finding(&f.source, actual, min);

    flagged
        .then(|| diagnostic(f, "fn_spacing", message, Measure { actual, limit }))
        .into_iter()
        .collect()
}

pub fn no_comments(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let rule = &cfg.strict_style.no_comments;
    if !rule.enabled {
        return Vec::new();
    }
    let comment_lines: Vec<usize> = f
        .comments
        .iter()
        .filter(|comment| match comment.kind {
            CommentKind::Ordinary => true,
            CommentKind::Documentation => !rule.allow_doc_comments,
            CommentKind::ToolDirective => !rule.allow_tool_directives,
        })
        .map(|comment| comment.line)
        .collect();
    if comment_lines.is_empty() {
        return Vec::new();
    }
    let actual = comment_lines.len();
    let lines: Vec<String> = comment_lines.iter().map(ToString::to_string).collect();
    let mut found = diagnostic(
        f,
        "no_comments",
        format!(
            "function `{}` has {actual} disallowed comment(s) on line {}; remove them or express their meaning in code",
            f.name,
            lines.join(", ")
        ),
        Measure { actual, limit: 0 },
    );
    found.details = vec![Detail {
        label: "comment".into(),
        count: actual,
        lines: comment_lines,
    }];
    vec![found]
}

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
    if rules.value_object.enabled {
        value_object::add(facts, &rules.value_object, &mut found);
    }
    found
}

#[cfg(test)]
mod tests;
