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
fn fn_length_and_fn_params_are_on_unless_turned_off() {
    let mut f = sample("f");
    f.source = format!("fn f() {{\n{}}}", "x();\n".repeat(60));
    f.params = 9;
    let mut cfg = Config::default();
    assert_eq!(fn_length(&f, &cfg)[0].rule_id, "fn_length");
    assert_eq!(fn_params(&f, &cfg)[0].rule_id, "fn_params");

    cfg.strict_style.fn_length.enabled = false;
    cfg.strict_style.fn_params.enabled = false;
    assert!(fn_length(&f, &cfg).is_empty());
    assert!(fn_params(&f, &cfg).is_empty());
}

#[test]
fn length_boundary() {
    let mut f = sample("f");
    let body = |lines| format!("fn f() {{\n{}}}", "x();\n".repeat(lines));
    f.source = body(48); // 50 code lines
    assert!(fn_length(&f, &Config::default()).is_empty());
    f.source = body(49);
    assert_eq!(fn_length(&f, &Config::default())[0].actual, 51);
}

#[test]
fn params_boundary_and_self() {
    let mut f = sample("f");
    f.params = 4;
    f.has_self = true;
    let mut cfg = Config::default();
    assert!(fn_params(&f, &cfg).is_empty());
    cfg.strict_style.fn_params.count_self = true;
    assert_eq!(fn_params(&f, &cfg)[0].actual, 5);
    f.params = 5;
    cfg.strict_style.fn_params.count_self = false;
    assert_eq!(fn_params(&f, &cfg)[0].actual, 5);
}
