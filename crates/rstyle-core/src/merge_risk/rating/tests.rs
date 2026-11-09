use super::super::GraphDelta;
use super::*;
use crate::config::CallRule;

fn change(name: &str, kind: ChangeKind) -> FunctionChange {
    FunctionChange {
        function: name.into(),
        name: name.into(),
        path: "src/lib.rs".into(),
        line: 7,
        kind,
        reason: None,
        signature_changed: false,
        values_changed: false,
        graph: GraphDelta::default(),
        calls_added: Vec::new(),
        calls_removed: Vec::new(),
        unresolved_calls: false,
        calls_unclassified: Vec::new(),
        effects_gained: Vec::new(),
        effects_lost: Vec::new(),
        callers: 0,
        reached_by: 0,
    }
}

fn changed(name: &str) -> FunctionChange {
    change(name, ChangeKind::Changed)
}

fn policy(pdg_change: Option<Severity>) -> RiskConfig {
    RiskConfig {
        pdg_change,
        ..RiskConfig::default()
    }
}

fn medium() -> RiskConfig {
    policy(Some(Severity::Medium))
}

fn calling_rule(callee: &str, level: Severity) -> CallRule {
    CallRule {
        callee: callee.into(),
        level,
    }
}

fn rated(changes: &[FunctionChange], policy: &RiskConfig) -> Rating {
    rate(
        &Measured {
            changes,
            changed_outside_functions: &[],
            not_compared_changed: 0,
        },
        policy,
    )
}

fn rules_at(rating: &Rating, level: Level) -> Vec<&'static str> {
    let at = rating.reasons.iter().filter(|r| r.level == level);
    at.map(|reason| reason.rule).collect()
}

#[test]
fn with_no_rules_anything_is_unrated_and_the_evidence_stays() {
    let rating = rated(
        &[change("text", ChangeKind::TextOnly), changed("f")],
        &RiskConfig::default(),
    );
    assert_eq!(rating.risk, Level::Unrated);
    assert!(!rating.policy_configured);
    assert_eq!(rules_at(&rating, Level::Unknown), [PDG_CHANGE]);
    assert_eq!(rules_at(&rating, Level::Unrated), [PDG_CHANGE]);

    let lines = rating.lines(2);
    assert_eq!(lines[0], "Merge risk: unrated 👀");
    assert!(lines[1].contains("no risk rules"), "{lines:?}");
}

#[test]
fn a_rule_for_something_else_leaves_a_change_unrated_and_says_what_is_missing() {
    let rules = RiskConfig {
        calls: vec![calling_rule("os.Remove", Severity::High)],
        ..RiskConfig::default()
    };
    let rating = rated(&[changed("f")], &rules);
    assert_eq!(rating.risk, Level::Unrated);
    assert!(rating.policy_configured);
    assert!(rating.lines(1)[1].contains("no severity for code impact"));
}

#[test]
fn nothing_changed_under_a_policy_is_low() {
    let rating = rated(&[], &medium());
    assert_eq!(rating.risk, Level::Low);
    assert_eq!(rating.lines(0)[0], "Merge risk: low ✅");
    assert!(
        rating
            .lines(0)
            .contains(&"Code impact: unchanged ✅".to_owned())
    );
}

#[test]
fn a_change_takes_the_severity_its_kind_is_given() {
    for severity in [Severity::Low, Severity::Medium, Severity::High] {
        for kind in [ChangeKind::Changed, ChangeKind::Added, ChangeKind::Removed] {
            let rating = rated(&[change("f", kind)], &policy(Some(severity)));
            assert_eq!(rating.risk, severity.into());
        }
    }
}

#[test]
fn what_the_compared_facts_cannot_speak_for_is_unknown_whatever_the_policy_says() {
    let low = policy(Some(Severity::Low));
    let text = rated(&[change("f", ChangeKind::TextOnly)], &low);
    assert_eq!(text.risk, Level::Unknown);

    let gap = FunctionChange {
        reason: Some("the compiler couldn't parse it"),
        ..change("f", ChangeKind::Unknown)
    };
    let rating = rated(&[gap], &low);
    assert_eq!(rating.risk, Level::Unknown);
    assert_eq!(rating.reasons[0].message, "the compiler couldn't parse it");
    assert!(
        rating
            .lines(1)
            .contains(&"Code impact: partial 👀".to_owned())
    );
}

#[test]
fn high_outweighs_unknown_which_outweighs_unrated_which_outweighs_medium() {
    let order = [
        Level::Low,
        Level::Medium,
        Level::Unrated,
        Level::Unknown,
        Level::High,
    ];
    for pair in order.windows(2) {
        assert_eq!(pair[0].strongest(pair[1]), pair[1]);
        assert_eq!(pair[1].strongest(pair[0]), pair[1]);
    }

    let writes = FunctionChange {
        calls_added: vec!["os.WriteFile".into()],
        ..changed("writes")
    };
    let rules = RiskConfig {
        pdg_change: Some(Severity::Low),
        calls: vec![calling_rule("os.WriteFile", Severity::High)],
        ..RiskConfig::default()
    };
    let rating = rated(&[change("text", ChangeKind::TextOnly), writes], &rules);
    assert_eq!(rating.risk, Level::High, "a measured high comes first");
}

#[test]
fn a_call_rule_names_one_callee_exactly_and_says_where() {
    let rules = RiskConfig {
        pdg_change: Some(Severity::Low),
        calls: vec![calling_rule("os.WriteFile", Severity::High)],
        ..RiskConfig::default()
    };
    let writes = |callee: &str| FunctionChange {
        calls_added: vec![callee.into()],
        ..changed("f")
    };

    let rating = rated(&[writes("os.WriteFile")], &rules);
    assert_eq!(rating.risk, Level::High);
    assert_eq!(rating.reasons[1].callee.as_deref(), Some("os.WriteFile"));
    let lines = rating.lines(1);
    assert_eq!(lines[0], "Merge risk: high ⚠️");
    assert_eq!(lines[1], "- src/lib.rs:7: f: added call: os.WriteFile");

    let other = rated(&[writes("os.WriteFileAt")], &rules);
    assert_eq!(other.risk, Level::Low);
}

#[test]
fn few_enough_functions_lower_a_medium_change_to_low() {
    let limited = RiskConfig {
        pdg_low_risk_max_functions: 2,
        ..medium()
    };
    let at_limit = rated(&[changed("a"), changed("b")], &limited);
    assert_eq!(at_limit.risk, Level::Low);
    assert!(at_limit.reasons[0].message.contains("low-risk limit of 2"));

    let over = rated(&[changed("a"), changed("b"), changed("c")], &limited);
    assert_eq!(over.risk, Level::Medium);
    assert_eq!(
        over.reasons[0].message,
        "3 functions changed, more than the low-risk limit of 2"
    );

    let no_limit = rated(&[changed("a")], &medium());
    assert_eq!(no_limit.risk, Level::Medium, "0 is no limit");
}

#[test]
fn the_size_limit_only_lowers_a_medium_change() {
    let high = RiskConfig {
        pdg_low_risk_max_functions: 5,
        ..policy(Some(Severity::High))
    };
    assert_eq!(rated(&[changed("a")], &high).risk, Level::High);

    let limited = RiskConfig {
        pdg_low_risk_max_functions: 5,
        calls: vec![calling_rule("os.Remove", Severity::Medium)],
        ..medium()
    };
    let removes = FunctionChange {
        calls_added: vec!["os.Remove".into()],
        ..changed("a")
    };
    assert_eq!(
        rated(&[removes], &limited).risk,
        Level::Medium,
        "a call rule"
    );

    let text = change("t", ChangeKind::TextOnly);
    assert_eq!(rated(&[text], &limited).risk, Level::Unknown, "a gap");

    let unresolved = FunctionChange {
        unresolved_calls: true,
        ..changed("a")
    };
    assert_eq!(
        rated(&[unresolved], &limited).risk,
        Level::Unknown,
        "a call"
    );
}

#[test]
fn a_call_that_cannot_be_read_keeps_a_policy_from_rating_it_low() {
    let low = policy(Some(Severity::Low));
    let unresolved = FunctionChange {
        unresolved_calls: true,
        ..changed("a")
    };
    let rating = rated(&[unresolved], &low);
    assert_eq!(rating.risk, Level::Unknown);
    assert_eq!(rules_at(&rating, Level::Unknown), [UNRESOLVED_CALLS]);

    let unclassified = FunctionChange {
        calls_unclassified: vec!["tokio::fs::write".into()],
        ..changed("a")
    };
    let rating = rated(&[unclassified], &low);
    assert_eq!(rating.risk, Level::Unknown);
    assert_eq!(rules_at(&rating, Level::Unknown), [UNCLASSIFIED_CALL]);
    assert_eq!(
        rating.reasons[1].callee.as_deref(),
        Some("tokio::fs::write")
    );
}

#[test]
fn a_rule_for_a_library_callee_is_the_project_saying_what_it_is() {
    let rules = RiskConfig {
        pdg_change: Some(Severity::Low),
        calls: vec![calling_rule("tokio::fs::write", Severity::Medium)],
        ..RiskConfig::default()
    };
    let writes = FunctionChange {
        calls_added: vec!["tokio::fs::write".into()],
        calls_unclassified: vec!["tokio::fs::write".into()],
        ..changed("a")
    };
    let rating = rated(&[writes], &rules);
    assert_eq!(rating.risk, Level::Medium);
    assert!(rules_at(&rating, Level::Unknown).is_empty());
}

#[test]
fn a_change_outside_what_is_compared_is_unknown() {
    let outside = rate(
        &Measured {
            changes: &[],
            changed_outside_functions: &["src/limits.rs".to_owned()],
            not_compared_changed: 0,
        },
        &medium(),
    );
    assert_eq!(outside.risk, Level::Unknown);
    assert_eq!(outside.reasons[0].rule, ANALYSIS_UNAVAILABLE);
    assert_eq!(outside.reasons[0].path.as_deref(), Some("src/limits.rs"));

    let generated = rate(
        &Measured {
            changes: &[],
            changed_outside_functions: &[],
            not_compared_changed: 4,
        },
        &medium(),
    );
    assert_eq!(generated.risk, Level::Unknown);
    assert!(generated.reasons[0].message.contains("4 functions"));
}

#[test]
fn what_cannot_be_seen_is_summed_up_by_kind() {
    let unclassified = FunctionChange {
        calls_unclassified: vec!["a".into(), "b".into()],
        unresolved_calls: true,
        ..changed("f")
    };
    let rating = rated(
        &[unclassified, change("t", ChangeKind::TextOnly)],
        &medium(),
    );
    assert_eq!(
        &rating.lines(2)[..4],
        [
            "Merge risk: unknown 👀",
            "- changes the compared facts can't speak for: 1",
            "- new library calls nothing classifies: 2",
            "- functions with a call to an unknown target: 1",
        ]
    );
}
