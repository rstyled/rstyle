//! Wire-shape invariants only; these fixtures never establish source permission.
use super::*;
use crate::copy_tagged_values::origins::{Alternative, Case, CaseKind, Condition};

fn wire() -> WireContract {
    WireContract {
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![],
        }],
        output_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![],
        }],
        alternatives: vec![Alternative {
            input: CaseKind::Whole,
            output: CaseKind::Whole,
            condition: Condition::Always,
            leaves: vec![],
        }],
    }
}

fn fixture() -> super::super::Summary {
    // Source verification is intentionally not claimed by this shape-only test.
    let facts: crate::HirFnFacts =
        serde_json::from_str(include_str!("../../copy_tagged_values/tests/0.json")).unwrap();
    summary(&facts, wire()).unwrap()
}

#[test]
fn named_summary_has_distinct_identity_and_keeps_the_complete_wire_vocabulary() {
    let actual = fixture();
    assert_eq!(actual.identity, SUMMARY);
    assert_eq!(validate_shape(&actual).unwrap(), &wire());
    assert!(super::super::tagged_conversions::validate_shape(&actual).is_err());
    let serialized = serde_json::to_string(&actual).unwrap();
    let replay: super::super::Summary = serde_json::from_str(&serialized).unwrap();
    assert!(actual == replay);
}

#[test]
fn missing_or_effectful_helper_summaries_cannot_claim_fresh_error_routes() {
    for change in 0..7 {
        let mut changed = fixture();
        match change {
            0 => changed.tagged_output = None,
            1 => changed.mutates_arguments = true,
            2 => changed.memory_effects = true,
            3 => changed.may_panic = true,
            4 => changed.always_returns = false,
            5 => changed.reference_outputs = Some(vec![]),
            _ => changed.component_outputs = Some(vec![]),
        }
        assert!(validate_shape(&changed).is_err(), "change {change}");
    }
}

#[test]
fn old_copy_summary_identity_cannot_be_reinterpreted_as_named_error_permission() {
    let mut changed = fixture();
    changed.identity = super::super::tagged_conversions::SUMMARY.into();
    assert!(validate_shape(&changed).is_err());
    assert!(super::super::tagged_conversions::validate_shape(&changed).is_ok());
}
