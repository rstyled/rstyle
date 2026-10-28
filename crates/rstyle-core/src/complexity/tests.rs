use super::*;

fn node(kind: Kind, children: Vec<Node>) -> Node {
    Node {
        kind,
        line: 1,
        column: 1,
        children,
    }
}

#[test]
fn nested_branches_cost_more_cognitive_than_flat_guards() {
    let nested = Measurement::score(&[node(Kind::Branch, vec![node(Kind::Branch, vec![])])]);
    let flat = Measurement::score(&[node(Kind::Branch, vec![]), node(Kind::Branch, vec![])]);
    assert_eq!((nested.cyclomatic, nested.cognitive), (3, 3));
    assert_eq!((flat.cyclomatic, flat.cognitive), (3, 2));
}

#[test]
fn selection_counts_alternatives_but_only_one_cognitive_branch() {
    let arms = (0..3).map(|_| node(Kind::Alternative, vec![])).collect();
    let score = Measurement::score(&[node(Kind::Selection, arms)]);
    assert_eq!((score.cyclomatic, score.cognitive), (3, 1));
}

#[test]
fn boolean_sequences_count_operator_runs() {
    let operators = [Kind::And, Kind::And, Kind::Or, Kind::And];
    let nodes = operators
        .into_iter()
        .map(|kind| node(kind, vec![]))
        .collect();
    let score = Measurement::score(&[node(Kind::Boolean, nodes)]);
    assert_eq!((score.cyclomatic, score.cognitive), (5, 3));
}

#[test]
fn propagation_and_labeled_jumps_have_distinct_costs() {
    let score = Measurement::score(&[node(Kind::Propagate, vec![]), node(Kind::Jump, vec![])]);
    assert_eq!((score.cyclomatic, score.cognitive), (2, 1));
}

#[test]
fn evidence_accounts_for_every_point() {
    let score = Measurement::score(&[node(Kind::Loop, vec![node(Kind::Branch, vec![])])]);
    assert_eq!(
        score.cyclomatic,
        1 + score
            .contributions
            .iter()
            .map(|c| c.cyclomatic)
            .sum::<usize>()
    );
    assert_eq!(
        score.cognitive,
        score
            .contributions
            .iter()
            .map(|c| c.cognitive)
            .sum::<usize>()
    );
    assert_eq!(score.contributions[1].nesting, 1);
}

#[test]
fn a_frontend_can_supply_the_shared_model_without_compiler_types() {
    let tree: Vec<Node> = serde_json::from_str(r#"[{"kind":"loop","line":2,"column":1,"children":[{"kind":"branch","line":3,"column":2,"children":[]}]}]"#).unwrap();
    let score = Measurement::score(&tree);
    assert_eq!((score.cyclomatic, score.cognitive), (3, 3));
}

#[test]
fn older_facts_preserve_unavailable_complexity() {
    let mut value = serde_json::to_value(crate::facts::sample("old")).unwrap();
    value.as_object_mut().unwrap().remove("complexity");
    let fact: crate::facts::FnFacts = serde_json::from_value(value).unwrap();
    let report = crate::evaluate(vec![fact.clone()], &crate::config::Config::default()).unwrap();
    assert!(report.functions[0].complexity.is_none());
    assert_eq!(report.summary.complexity_measured, 0);
    assert_eq!(report.summary.complexity_unavailable, 1);
    assert_eq!(report.exit_code(), 2);
    let config = crate::config::Config::parse("[complexity]\nmax_cognitive = 5\n").unwrap();
    let report = crate::evaluate(vec![fact], &config).unwrap();
    assert_eq!(report.exit_code(), 2);
    assert!(crate::report::text(&report).contains("?    ?"));
    assert!(crate::report::text(&report).contains("0 measured, 1 unavailable"));
}

#[test]
fn partial_measurements_keep_other_findings_and_fail_the_report() {
    let unknown = crate::facts::sample("unknown");
    let mut measured = crate::facts::sample("measured");
    measured.params = 5;
    measured.complexity = Some(Measurement::score(&[]));
    let report =
        crate::evaluate(vec![unknown, measured], &crate::config::Config::default()).unwrap();
    assert_eq!(report.functions.len(), 2);
    assert_eq!(report.summary.complexity_measured, 1);
    assert_eq!(report.summary.complexity_unavailable, 1);
    assert_eq!(report.diagnostics[0].rule_id, "fn_params");
    assert_eq!(report.exit_code(), 2);
    let text = crate::report::text(&report);
    assert!(text.contains("fn_params"));
    assert!(text.contains("?    ?"));
}

#[test]
fn closure_nesting_and_else_if_costs_are_shared_policy() {
    let branches = vec![
        node(Kind::Branch, vec![]),
        node(Kind::ElseIf, vec![node(Kind::Branch, vec![])]),
        node(Kind::Else, vec![]),
    ];
    let score = Measurement::score(&[node(Kind::Closure, branches)]);
    assert_eq!((score.cyclomatic, score.cognitive), (4, 7));
}
