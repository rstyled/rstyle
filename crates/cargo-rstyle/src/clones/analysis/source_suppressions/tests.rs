use super::*;

fn report(comment: &str) -> Report {
    evaluate(&Availability::Known(comment.into()))
}

#[test]
fn named_source_rules_require_a_valid_complete_comment() {
    let allowed = report("// rstyle-allow(fn_params): ffi shim");
    assert!(allowed.complete());
    assert!(allowed.allows("fn_params"));
    assert!(!allowed.allows("fn_length"));
    assert!(allowed.invalid_assessment().is_none());
    let both = report("  // rstyle-allow(fn_length, fn_params): generated");
    assert!(both.allows("fn_length"));
    assert!(both.allows("fn_params"));
}

#[test]
fn valid_legacy_rules_do_not_suppress_unmigrated_consumers() {
    let allowed = report(
        "// rstyle-allow(fn_length, fn_spacing, mutation_per_target, mutated_targets, side_effect_density, abstraction): deliberate",
    );
    assert!(allowed.complete());
    assert!(allowed.allows("fn_length"));
    assert!(allowed.allows("fn_spacing"));
    for rule in suppress::RULE_IDS {
        if !matches!(*rule, "fn_length" | "fn_params" | "fn_spacing") {
            assert!(!allowed.allows(rule), "{rule}");
        }
    }
}

#[test]
fn empty_and_plain_comments_are_complete_without_permission() {
    for comment in ["", "// ordinary note"] {
        let value = report(comment);
        assert!(matches!(&value, Report::None { comment: saved } if saved.as_str() == comment));
        assert!(value.complete());
        assert!(!value.allows("fn_length"));
        assert!(!value.allows("fn_params"));
        assert!(value.invalid_assessment().is_none());
    }
}

#[test]
fn unknown_names_invalidate_the_entire_comment() {
    let value = report("// rstyle-allow(fn_length, unknown): deliberate");
    assert!(!value.allows("fn_length"));
    assert!(value.complete());
    let assessment = value.invalid_assessment().unwrap();
    assert_eq!(assessment.rule_id, "invalid_suppression");
    assert_eq!(assessment.outcome, Outcome::Finding);
    assert_eq!(assessment.coverage, Coverage::Complete);
    assert_eq!(assessment.availability, FactAvailability::Known);
    assert_eq!(assessment.actual, Some(1));
    assert_eq!(assessment.limit, 0);
    assert_eq!(
        assessment.reasons,
        ["unknown rule `unknown` in rstyle-allow"]
    );
}

#[test]
fn malformed_comments_retain_the_parser_reason_and_original_line() {
    for comment in [
        "// rstyle-allow(fn_params)",
        "// rstyle-allow(fn_params):  ",
        "// rstyle-allow(fn_params: why",
    ] {
        let value = report(comment);
        let Suppression::Invalid(expected) = suppress::parse(comment) else {
            panic!("fixture must be invalid");
        };
        assert!(
            matches!(&value, Report::Invalid { comment: saved, reason } if saved.as_str() == comment && reason == &expected)
        );
        assert_eq!(value.invalid_assessment().unwrap().reasons, [expected]);
        assert!(!value.allows("fn_params"));
    }
}

#[test]
fn unavailable_comments_preserve_reasons_without_permission_or_findings() {
    for input in [
        Availability::Unknown {
            reason: "missing".into(),
        },
        Availability::Unsupported {
            reason: "expanded".into(),
        },
        Availability::NotApplicable {
            reason: "foreign".into(),
        },
    ] {
        let expected = match &input {
            Availability::Unknown { reason }
            | Availability::Unsupported { reason }
            | Availability::NotApplicable { reason } => reason,
            Availability::Known(_) => unreachable!(),
        };
        let value = evaluate(&input);
        assert!(matches!(&value, Report::Unavailable { reason } if reason == expected));
        assert!(!value.complete());
        assert!(!value.allows("fn_params"));
        assert!(!value.allows("fn_length"));
        assert!(value.invalid_assessment().is_none());
    }
    assert!(!Report::failure("failed".into()).complete());
}

#[test]
fn mutation_permissions_are_named_and_disjoint_from_source_permissions() {
    let one = report("// rstyle-allow(mutation_per_target): deliberate");
    assert!(one.allows_mutation("mutation_per_target"));
    assert!(!one.allows_mutation("mutated_targets"));
    assert!(!one.allows("mutation_per_target"));
    let mixed =
        report("// rstyle-allow(fn_length, mutated_targets, side_effect_density): deliberate");
    assert!(mixed.allows("fn_length"));
    assert!(mixed.allows_mutation("mutated_targets"));
    assert!(!mixed.allows_mutation("fn_length"));
    assert!(!mixed.allows_mutation("side_effect_density"));
    assert!(!mixed.allows_mutation("abstraction"));
}

#[test]
fn invalid_or_unavailable_comments_cannot_grant_mutation_permissions() {
    for value in [
        report("// rstyle-allow(mutation_per_target):"),
        report("// rstyle-allow(mutated_targets, unknown): deliberate"),
        Report::failure("missing".into()),
    ] {
        assert!(!value.allows_mutation("mutation_per_target"));
        assert!(!value.allows_mutation("mutated_targets"));
    }
}
