use super::*;

fn assessed(source: &str) -> GuardAssessment {
    evaluate(&Availability::Known(source))
}

#[test]
fn frozen_nested_shape_has_a_scalar_fallback_and_two_continuing_statements() {
    let result = assessed(
        "fn nested_positive(value: u32) -> u32 { if value > 0 { let next = value + 1; next + 1 } else { 0 } }",
    );
    assert_eq!(result.outcome, Outcome::Finding);
    assert_eq!(result.coverage, Coverage::Complete);
    let opportunity = result.opportunity.unwrap();
    assert_eq!(opportunity.fallback_branch, Branch::Else);
    assert_eq!(opportunity.continuing_statements, 2);
    assert!(opportunity.advisory.contains("review lifetimes, drops"));
}

#[test]
fn direct_return_and_scalar_then_branch_are_supported_advisory_shapes() {
    for fallback in ["return 0;", "0"] {
        let source = format!(
            "fn f(value: u32) -> u32 {{ if value == 0 {{ {fallback} }} else {{ let x = value + 1; x + 1 }} }}"
        );
        let result = assessed(&source);
        assert_eq!(result.outcome, Outcome::Finding);
        assert_eq!(result.opportunity.unwrap().fallback_branch, Branch::Then);
    }
}

#[test]
fn flat_and_both_work_and_small_branches_have_no_opportunity() {
    for source in [
        "fn already_flat(value: u32) -> u32 { if value == 0 { return 0; } let next = value + 1; next + 1 }",
        "fn both(value: u32) -> u32 { if value > 4 { let next = value + 1; next + 1 } else { let previous = value + 2; previous + 2 } }",
        "fn small(value: u32) -> u32 { if value > 0 { value + 1 } else { 0 } }",
        "fn path(value: u32) -> u32 { if value > 0 { let x = value + 1; x } else { value } }",
    ] {
        let result = assessed(source);
        assert_eq!(result.outcome, Outcome::EvaluatedNoFinding, "{source}");
        assert!(result.opportunity.is_none());
    }
}

#[test]
fn opaque_or_ambiguous_syntax_is_not_evaluated_as_clean() {
    for body in [
        "if value > 0 { let x = opaque!(); x } else { 0 }",
        "if value > 0 { let x = unsafe { value + 1 }; x } else { 0 }",
        "if value > 0 { let x = || value + 1; x() } else { 0 }",
        "if value > 0 { let x = value + 1; x } else if value == 0 { 0 } else { 1 }",
        "'label: { if value > 0 { let x = value + 1; x } else { 0 } }",
        "loop { break if value > 0 { let x = value + 1; x } else { 0 }; }",
        "if value > 0 { #[cfg(any())] let x = 1; value } else { 0 }",
    ] {
        let source = format!("fn f(value: u32) -> u32 {{ {body} }}");
        let result = assessed(&source);
        assert_eq!(result.outcome, Outcome::NotEvaluated, "{body}");
        assert_eq!(result.availability, FactAvailability::Unsupported);
        assert_eq!(result.coverage, Coverage::Incomplete);
        assert!(!result.reasons.is_empty());
    }
}

#[test]
fn conditional_returns_do_not_gain_an_opportunity() {
    for source in [
        "fn f(value: u32) -> u32 { if value > 0 { if value == 1 { return 4; } value + 1 } else { 0 } }",
        "fn f(value: u32) -> u32 { return 0; if value > 0 { let x = value + 1; x } else { 0 } }",
        "fn f(value: u32) -> u32 { if return 0 { let x = value + 1; x } else { 0 } }",
    ] {
        let result = assessed(source);
        assert_eq!(result.outcome, Outcome::EvaluatedNoFinding);
        assert!(result.opportunity.is_none());
    }
}

#[test]
fn unavailable_source_preserves_specific_status_and_reason() {
    let cases = [
        (
            Availability::Unknown {
                reason: "source absent".into(),
            },
            FactAvailability::Unknown,
        ),
        (
            Availability::Unsupported {
                reason: "expanded source".into(),
            },
            FactAvailability::Unsupported,
        ),
        (
            Availability::NotApplicable {
                reason: "no Rust body".into(),
            },
            FactAvailability::NotApplicable,
        ),
    ];
    for (source, expected) in cases {
        let result = evaluate(&source);
        assert_eq!(result.outcome, Outcome::NotEvaluated);
        assert_eq!(result.availability, expected);
        assert_eq!(result.coverage, Coverage::Incomplete);
        assert_eq!(result.reasons.len(), 1);
    }
}

#[test]
fn source_byte_budget_is_explicit_without_truncating_or_declaring_clean() {
    let source = format!("fn f() {{ /*{}*/ }}", "x".repeat(256 * 1024));
    let result = assessed(&source);
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert_eq!(result.outcome, Outcome::NotEvaluated);
    assert_eq!(result.availability, FactAvailability::Known);
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert!(result.opportunity.is_none());
    assert!(result.reasons[0].contains("budget exhausted"));
}

#[test]
fn recursive_and_flat_syntax_exhausts_before_guard_parsing() {
    for body in [
        format!("{}true", "!".repeat(1000)),
        format!("{}0", "return ".repeat(1000)),
        format!("{}0{}", "(".repeat(129), ")".repeat(129)),
        format!("{}0", "1 + ".repeat(129)),
    ] {
        let source = format!("fn f() {{ {body} }}");
        let result = assessed(&source);
        assert_eq!(result.execution, Execution::BudgetExhausted);
        assert_eq!(result.outcome, Outcome::NotEvaluated);
        assert_eq!(result.availability, FactAvailability::Known);
        assert_eq!(result.coverage, Coverage::Incomplete);
        assert!(result.opportunity.is_none());
    }
}

#[test]
fn opportunity_span_is_exact_final_if_in_original_utf8_crlf_bytes() {
    let branch = "if π == 0 { 0 } else { let next = π + 1; next }";
    let source = format!(
        "fn café(π: u32) -> u32 {{\r\n    let prior = π;\r\n    {branch} /* after */\r\n}}"
    );
    let opportunity = assessed(&source).opportunity.unwrap();
    let start = source.find("if π").unwrap();
    assert_eq!(opportunity.span.start as usize, start);
    assert_eq!(opportunity.span.end as usize, start + branch.len());
    assert_eq!(
        &source[opportunity.span.start as usize..opportunity.span.end as usize],
        branch
    );
}
