use super::*;
use rstyle_pdg::consumer_facts::Signature;

fn facts(source: &str, parameters: usize, has_self: bool) -> LocalFacts<'_> {
    LocalFacts {
        source: Availability::Known(source),
        signature: Availability::Known(Signature {
            parameters,
            has_self,
        }),
    }
}

#[test]
fn code_line_boundary_ignores_blank_and_comment_only_lines() {
    let mut config = Config::default();
    config.fn_length.max_lines = 3;
    let clean = "fn sample() {\n// note\n\n/* nested /* note */ */\nx();\n}";
    let assessed = evaluate(&facts(clean, 0, false), &config);
    assert_eq!(assessed[0].actual, Some(3));
    assert_eq!(assessed[0].outcome, Outcome::EvaluatedNoFinding);
    assert_eq!(assessed[0].coverage, Coverage::Complete);

    let long = "fn sample() {\nx();\ny();\n}";
    let assessed = evaluate(&facts(long, 0, false), &config);
    assert_eq!(assessed[0].actual, Some(4));
    assert_eq!(assessed[0].limit, 3);
    assert_eq!(assessed[0].outcome, Outcome::Finding);
}

#[test]
fn parameter_boundary_and_receiver_policy_follow_legacy_configuration() {
    let mut config = Config::default();
    let method = facts("fn method(&self, a: u8, b: u8, c: u8, d: u8) {}", 4, true);
    let assessed = evaluate(&method, &config);
    assert_eq!(assessed[1].actual, Some(4));
    assert_eq!(assessed[1].outcome, Outcome::EvaluatedNoFinding);

    config.fn_params.count_self = true;
    let assessed = evaluate(&method, &config);
    assert_eq!(assessed[1].actual, Some(5));
    assert_eq!(assessed[1].outcome, Outcome::Finding);
    let free = evaluate(&facts("fn free() {}", 5, false), &config);
    assert_eq!(free[1].actual, Some(5));
    assert_eq!(free[1].outcome, Outcome::Finding);
}

#[test]
fn unsupported_body_does_not_preclude_source_and_signature_budgets() {
    let mut config = Config::default();
    config.fn_length.max_lines = 2;
    config.fn_params.max_params = 1;
    let source = "fn opaque(a: u8, b: u8) {\nlet call = || unknown!();\ncall();\n}";
    let assessed = evaluate(&facts(source, 2, false), &config);
    assert!(assessed.iter().all(|rule| rule.outcome == Outcome::Finding));
    assert!(
        assessed
            .iter()
            .all(|rule| rule.coverage == Coverage::Complete)
    );
}

#[test]
fn unsupported_and_unknown_remain_separate_unevaluated_reasons() {
    let unavailable = LocalFacts {
        source: Availability::Unknown {
            reason: "original source unavailable".into(),
        },
        signature: Availability::Unsupported {
            reason: "variadic parameter list".into(),
        },
    };
    let assessed = evaluate(&unavailable, &Config::default());
    assert_eq!(assessed[0].availability, FactAvailability::Unknown);
    assert_eq!(assessed[1].availability, FactAvailability::Unsupported);
    assert_eq!(assessed[0].reasons, ["original source unavailable"]);
    assert_eq!(assessed[1].reasons, ["variadic parameter list"]);
    assert!(assessed.iter().all(|rule| rule.actual.is_none()));
    assert!(
        assessed
            .iter()
            .all(|rule| rule.outcome == Outcome::NotEvaluated)
    );
    assert!(
        assessed
            .iter()
            .all(|rule| rule.coverage == Coverage::Incomplete)
    );
}

#[test]
fn known_empty_differs_from_inapplicable_and_one_finding_can_survive_missing_fact() {
    let unavailable = LocalFacts {
        source: Availability::NotApplicable {
            reason: "no original Rust function source".into(),
        },
        signature: Availability::Known(Signature {
            parameters: 5,
            has_self: false,
        }),
    };
    let assessed = evaluate(&unavailable, &Config::default());
    assert_eq!(assessed[0].availability, FactAvailability::NotApplicable);
    assert_eq!(assessed[0].outcome, Outcome::NotEvaluated);
    assert_eq!(assessed[1].outcome, Outcome::Finding);

    let empty = evaluate(&facts("", 0, false), &Config::default());
    assert!(empty.iter().all(|rule| rule.actual == Some(0)));
    assert!(
        empty
            .iter()
            .all(|rule| rule.outcome == Outcome::EvaluatedNoFinding)
    );
}
