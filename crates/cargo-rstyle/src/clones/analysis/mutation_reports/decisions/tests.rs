use super::*;
use rstyle_pdg::consumer_facts::Availability;

fn included() -> source_exclusions::Report {
    source_exclusions::Report::Included {
        path: "src/lib.rs".into(),
    }
}

fn excluded() -> source_exclusions::Report {
    source_exclusions::Report::Excluded {
        path: "vendor/lib.rs".into(),
    }
}

fn suppression(comment: &str) -> source_suppressions::Report {
    source_suppressions::evaluate(&Availability::Known(comment.into()))
}

fn findings() -> Vec<Rule> {
    vec![
        Rule {
            rule_id: "mutation_per_target",
            outcome: Outcome::Finding,
            actual: Some(3),
            limit: 2,
        },
        Rule {
            rule_id: "mutated_targets",
            outcome: Outcome::Finding,
            actual: Some(2),
            limit: 1,
        },
    ]
}

fn status(complete: bool) -> RawStatus {
    RawStatus {
        complete,
        execution: Execution::Completed,
    }
}

#[test]
fn named_suppression_only_hides_its_original_rule_and_retains_measurement() {
    let raw = findings();
    let policy = evaluate(
        included(),
        suppression("// rstyle-allow(mutation_per_target): reviewed"),
        &raw,
        status(true),
    );
    assert_eq!(policy.coverage, Coverage::Complete);
    assert_eq!(policy.counts().visible, 1);
    assert_eq!(policy.counts().suppressed, 1);
    assert_eq!(policy.counts().excluded, 0);
    assert_eq!(
        policy.rule_decisions[0].disposition,
        Disposition::Suppressed
    );
    assert_eq!(policy.rule_decisions[0].outcome, Outcome::NotEvaluated);
    assert_eq!(policy.rule_decisions[0].actual, Some(3));
    assert_eq!(policy.rule_decisions[1].disposition, Disposition::Evaluated);
    assert_eq!(raw[0].outcome, Outcome::Finding);
    assert_eq!(raw[0].actual, Some(3));
}

#[test]
fn unrelated_valid_rules_never_suppress_mutation_findings() {
    let policy = evaluate(
        included(),
        suppression("// rstyle-allow(fn_length, side_effect_density, abstraction): reviewed"),
        &findings(),
        status(true),
    );
    assert_eq!(policy.counts().visible, 2);
    assert_eq!(policy.counts().suppressed, 0);
    assert!(
        policy
            .rule_decisions
            .iter()
            .all(|rule| rule.disposition == Disposition::Evaluated)
    );
}

#[test]
fn suppressing_both_rules_cannot_hide_incomplete_original_coverage() {
    let policy = evaluate(
        included(),
        suppression("// rstyle-allow(mutation_per_target, mutated_targets): reviewed"),
        &findings(),
        status(false),
    );
    assert_eq!(policy.counts().visible, 0);
    assert_eq!(policy.counts().suppressed, 2);
    assert_eq!(policy.coverage, Coverage::Incomplete);
    assert_eq!(policy.execution, Execution::Completed);
    assert!(!policy.excluded());
}

#[test]
fn exclusion_is_an_explicit_complete_skip_without_measured_zero() {
    let policy = evaluate(
        excluded(),
        source_suppressions::Report::failure("comment not captured".into()),
        &findings(),
        RawStatus {
            complete: false,
            execution: Execution::BudgetExhausted,
        },
    );
    assert_eq!(policy.coverage, Coverage::Complete);
    assert_eq!(policy.execution, Execution::Completed);
    assert!(policy.excluded());
    assert_eq!(policy.counts().visible, 0);
    assert_eq!(policy.counts().excluded, 2);
    assert!(
        policy
            .rule_decisions
            .iter()
            .all(|rule| rule.disposition == Disposition::Excluded
                && rule.actual.is_none()
                && rule.outcome == Outcome::NotEvaluated)
    );
}

#[test]
fn invalid_annotation_is_visible_even_when_body_rules_are_excluded() {
    let policy = evaluate(
        excluded(),
        suppression("// rstyle-allow(mutation_per_target, unknown): reviewed"),
        &findings(),
        status(false),
    );
    assert_eq!(policy.coverage, Coverage::Complete);
    assert_eq!(policy.counts().visible, 1);
    assert_eq!(policy.counts().excluded, 2);
    let invalid = policy.invalid_suppression.as_ref().unwrap();
    assert_eq!(invalid.rule_id, "invalid_suppression");
    assert_eq!(invalid.outcome, Outcome::Finding);
    assert_eq!(invalid.reasons, ["unknown rule `unknown` in rstyle-allow"]);
}

#[test]
fn invalid_annotation_never_grants_partial_named_permission() {
    let policy = evaluate(
        included(),
        suppression("// rstyle-allow(mutation_per_target, unknown): reviewed"),
        &findings(),
        status(true),
    );
    assert_eq!(policy.counts().visible, 3);
    assert_eq!(policy.counts().suppressed, 0);
    assert_eq!(policy.coverage, Coverage::Complete);
}

#[test]
fn unavailable_policy_preserves_raw_positive_witnesses() {
    for missing in ["path", "comment"] {
        let exclusion = if missing == "path" {
            source_exclusions::Report::Unavailable {
                reason: "missing source path".into(),
            }
        } else {
            included()
        };
        let comment = if missing == "comment" {
            source_suppressions::Report::failure("comment unavailable".into())
        } else {
            suppression("")
        };
        let policy = evaluate(exclusion, comment, &findings(), status(true));
        assert_eq!(policy.coverage, Coverage::Incomplete);
        assert_eq!(policy.counts().visible, 2);
        assert!(
            policy
                .rule_decisions
                .iter()
                .all(|rule| rule.disposition == Disposition::Unavailable
                    && rule.outcome == Outcome::Finding
                    && rule.actual.is_some())
        );
    }
}

#[test]
fn budget_execution_survives_suppression_and_unavailable_decisions() {
    for comment in [
        "",
        "// rstyle-allow(mutation_per_target, mutated_targets): reviewed",
    ] {
        let policy = evaluate(
            included(),
            suppression(comment),
            &findings(),
            RawStatus {
                complete: false,
                execution: Execution::BudgetExhausted,
            },
        );
        assert_eq!(policy.coverage, Coverage::Incomplete);
        assert_eq!(policy.execution, Execution::BudgetExhausted);
    }
}

#[test]
fn outside_workspace_is_included_and_exact_zero_is_evaluated() {
    let raw = [Rule {
        rule_id: "mutated_targets",
        outcome: Outcome::EvaluatedNoFinding,
        actual: Some(0),
        limit: 1,
    }];
    let policy = evaluate(
        source_exclusions::Report::NotApplicable {
            path: "/other/lib.rs".into(),
            reason: "outside workspace".into(),
        },
        suppression(""),
        &raw,
        status(true),
    );
    assert!(!policy.excluded());
    assert_eq!(policy.coverage, Coverage::Complete);
    assert_eq!(policy.rule_decisions[0].disposition, Disposition::Evaluated);
    assert_eq!(policy.rule_decisions[0].actual, Some(0));
    assert_eq!(
        policy.rule_decisions[0].outcome,
        Outcome::EvaluatedNoFinding
    );
}

#[test]
fn serialized_policy_distinguishes_skip_from_zero_without_internal_counts() {
    let policy = evaluate(excluded(), suppression(""), &findings(), status(true));
    let json = serde_json::to_value(policy).unwrap();
    assert_eq!(json["rule_decisions"][0]["disposition"], "excluded");
    assert!(json["rule_decisions"][0]["actual"].is_null());
    assert!(json["rule_decisions"][0].get("witnessed_finding").is_none());
    assert_eq!(json["source_exclusion"]["status"], "excluded");
}
