use super::*;
use crate::facts::sample;

fn measured(name: &str, cognitive: usize, cyclomatic: usize) -> FnFacts {
    FnFacts {
        complexity: Some(Measurement {
            policy_version: 1,
            cyclomatic,
            cognitive,
            contributions: vec![],
        }),
        ..sample(name)
    }
}

fn names<'a>(functions: &[&'a FnFacts]) -> Vec<&'a str> {
    functions.iter().map(|fact| fact.name.as_str()).collect()
}

#[test]
fn ranking_borrows_every_clean_measurement_without_creating_findings() {
    let facts = [measured("flat", 0, 1), measured("branch", 1, 2)];
    let functions = ranked(&facts);
    assert_eq!(names(&functions), ["branch", "flat"]);
    assert!(std::ptr::eq(functions[0], &facts[1]));
    assert!(std::ptr::eq(functions[1], &facts[0]));
    assert!(findings(&facts, &[]).is_empty());
}

#[test]
fn cognitive_then_cyclomatic_scores_precede_unavailable_owners() {
    let facts = [
        sample("unknown"),
        measured("high_cc", 1, 20),
        measured("high_cog", 2, 3),
        measured("tie_cog", 2, 5),
        measured("zero_cog", 0, 1),
    ];
    assert_eq!(
        names(&ranked(&facts)),
        ["tie_cog", "high_cog", "high_cc", "zero_cog", "unknown"]
    );
    assert_eq!(facts[0].complexity, None);
    let mut completed: Vec<_> = facts.iter().map(Function::from).collect();
    completed.sort_by(ordering);
    let expected: Vec<_> = ranked(&facts).into_iter().map(Function::from).collect();
    assert_eq!(completed, expected);
    let rows = completed.iter().collect::<Vec<_>>();
    assert_eq!(text_refs(rows), text(&completed));
    assert!(text_refs([]).is_empty());
}

fn located(name: &str, path: &str, line: usize, column: usize) -> FnFacts {
    FnFacts {
        path: path.into(),
        line,
        column,
        ..measured(name, 1, 2)
    }
}

#[test]
fn score_ties_use_path_line_column_and_name_for_known_and_unknown_rows() {
    let mut facts = [
        located("path", "z.rs", 1, 1),
        located("line", "a.rs", 2, 1),
        located("column", "a.rs", 1, 2),
        located("beta", "a.rs", 1, 1),
        located("alpha", "a.rs", 1, 1),
    ];
    let expected = ["alpha", "beta", "column", "line", "path"];
    assert_eq!(names(&ranked(&facts)), expected);
    for fact in &mut facts {
        fact.complexity = None;
    }
    assert_eq!(names(&ranked(&facts)), expected);
}

fn diagnostic(fact: &FnFacts) -> Diagnostic {
    Diagnostic {
        path: fact.path.clone(),
        line: fact.line,
        column: fact.column,
        rule_id: "fn_cognitive_complexity".into(),
        message: "complexity finding".into(),
        actual: 2,
        limit: 1,
        details: vec![],
    }
}

#[test]
fn findings_keep_diagnostic_and_unavailable_rows_in_the_existing_order() {
    let facts = [
        FnFacts {
            line: 4,
            ..sample("unknown")
        },
        FnFacts {
            line: 3,
            ..measured("clean", 5, 6)
        },
        FnFacts {
            line: 2,
            ..measured("finding", 2, 3)
        },
        FnFacts {
            line: 1,
            ..measured("other_finding", 1, 9)
        },
    ];
    let diagnostics = [diagnostic(&facts[2]), diagnostic(&facts[3])];
    assert_eq!(
        findings(&facts, &diagnostics),
        vec![
            Function::from(&facts[2]),
            Function::from(&facts[3]),
            Function::from(&facts[0])
        ]
    );
    assert_eq!(
        names(&ranked(&facts)),
        ["clean", "finding", "other_finding", "unknown"]
    );
}
