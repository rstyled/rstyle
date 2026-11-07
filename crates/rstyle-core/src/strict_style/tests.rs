use super::*;
use crate::facts::{GuardClause, sample};

fn config(on: bool) -> Config {
    let mut config = Config::default();
    config.strict_style.guard_clause = on;
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
