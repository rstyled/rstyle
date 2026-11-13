use super::*;
use crate::facts::{GuardClause, sample};

fn config(on: bool) -> Config {
    let mut config = Config::default();
    config.strict_style.guard_clause.enabled = on;
    config
}

fn with_guards(lines: &[usize]) -> FnFacts {
    let mut f = sample("f");
    f.guard_clauses = lines
        .iter()
        .map(|&line| GuardClause {
            line,
            statements: 3,
        })
        .collect();
    f
}

#[test]
fn a_rule_that_is_off_reports_nothing() {
    assert!(guard_clause(&with_guards(&[3]), &config(false)).is_empty());
}

#[test]
fn a_rule_that_is_on_says_where_at_the_function() {
    let f = with_guards(&[3, 9]);
    let [found] = &guard_clause(&f, &config(true))[..] else {
        panic!("one diagnostic for the function");
    };
    assert_eq!(found.rule_id, "guard_clause");
    assert_eq!((found.line, found.column), (f.line, f.column));
    assert!(found.actual > found.limit);
    assert!(found.message.contains("line 3, 9"), "{}", found.message);
    assert_eq!(found.details[0].lines, [3, 9]);
}

#[test]
fn a_function_with_nothing_to_guard_is_clean() {
    assert!(guard_clause(&with_guards(&[]), &config(true)).is_empty());
}

#[test]
fn a_project_says_how_much_nesting_is_a_main_path() {
    let mut f = with_guards(&[3, 9]);
    f.guard_clauses[0].statements = 1;
    let reported = |limit: usize| {
        let mut config = config(true);
        config.strict_style.guard_clause.min_nested_statements = limit;
        let found = guard_clause(&f, &config);
        found.first().map(|found| found.details[0].lines.clone())
    };

    assert_eq!(
        reported(2),
        Some(vec![9]),
        "one statement isn't, by default"
    );
    assert_eq!(reported(1), Some(vec![3, 9]));
    assert_eq!(reported(0), Some(vec![3, 9]), "0 reads as 1");
    assert_eq!(reported(4), None);
}

fn spacing(on: bool) -> Config {
    let mut config = Config::default();
    config.strict_style.fn_spacing.enabled = on;
    config
}

#[test]
fn fn_spacing_is_off_unless_asked_for() {
    let mut f = sample("f");
    f.top_statements = 20;
    assert!(fn_spacing(&f, &spacing(false)).is_empty());
    assert_eq!(fn_spacing(&f, &spacing(true)).len(), 1);
}

#[test]
fn spacing_boundary_and_blank_line() {
    let mut f = sample("f");
    let cfg = spacing(true);
    f.top_statements = 4;
    assert!(fn_spacing(&f, &cfg).is_empty());
    f.top_statements = 5;
    let out = fn_spacing(&f, &cfg);
    assert_eq!(out[0].rule_id, "fn_spacing");
    assert_eq!((out[0].actual, out[0].limit), (5, 4));
    f.source = "fn f() {\nx();\n\nx();\n}".into();
    assert!(fn_spacing(&f, &cfg).is_empty());
}

#[test]
fn long_initializers_do_not_count() {
    let mut f = sample("f");
    f.source = format!("fn f() {{\n{}}}", "x();\n".repeat(40));
    f.top_statements = 3;
    assert!(fn_spacing(&f, &spacing(true)).is_empty());
    f.top_statements = 0; // facts from older runs
    assert!(fn_spacing(&f, &spacing(true)).is_empty());
}

#[test]
fn a_project_says_how_many_statements_need_a_blank_line() {
    let mut f = sample("f");
    f.top_statements = 8;
    let mut cfg = spacing(true);
    cfg.strict_style.fn_spacing.min_statements = 10;
    assert!(fn_spacing(&f, &cfg).is_empty());
    cfg.strict_style.fn_spacing.min_statements = 8;
    assert_eq!(fn_spacing(&f, &cfg).len(), 1);
}

#[test]
fn no_comments_is_opt_in_and_reports_source_lines() {
    let mut f = sample("commented");
    f.comments = [4, 7]
        .into_iter()
        .map(|line| crate::facts::SourceComment {
            line,
            kind: CommentKind::Ordinary,
        })
        .collect();
    let mut cfg = Config::default();
    assert!(no_comments(&f, &cfg).is_empty());
    cfg.strict_style.no_comments.enabled = true;
    let found = no_comments(&f, &cfg);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule_id, "no_comments");
    assert_eq!((found[0].actual, found[0].limit), (2, 0));
    assert_eq!(found[0].details[0].lines, [4, 7]);
    f.comments.clear();
    assert!(no_comments(&f, &cfg).is_empty());
}

#[test]
fn comment_exceptions_are_independent_and_default_to_allowed() {
    let mut f = sample("comments");
    f.comments = [
        CommentKind::Ordinary,
        CommentKind::Documentation,
        CommentKind::ToolDirective,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| crate::facts::SourceComment {
        line: index + 2,
        kind,
    })
    .collect();
    for (docs, directives, expected) in [
        (true, true, vec![2]),
        (false, true, vec![2, 3]),
        (true, false, vec![2, 4]),
        (false, false, vec![2, 3, 4]),
    ] {
        let mut cfg = Config::default();
        cfg.strict_style.no_comments = crate::config::NoCommentsConfig {
            enabled: true,
            allow_doc_comments: docs,
            allow_tool_directives: directives,
        };
        let found = no_comments(&f, &cfg);
        assert_eq!(found[0].details[0].lines, expected);
    }
}
