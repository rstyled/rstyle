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
