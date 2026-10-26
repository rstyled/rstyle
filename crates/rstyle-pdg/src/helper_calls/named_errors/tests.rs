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

#[test]
fn scalar_constructor_contract_is_distinct_from_historical_unary_conversion() {
    let mut scalar = fixture();
    scalar.argument_types = vec!["u32".into(), "u32".into()];
    assert!(validate_shape(&scalar).is_ok());
    assert!(super::super::tagged_conversions::validate_shape_as(&scalar, SUMMARY).is_err());
    for arguments in [
        vec!["u32"],
        vec!["u32", "u64"],
        vec!["usize", "usize"],
        vec!["u32", "u32", "u32"],
    ] {
        let mut changed = scalar.clone();
        changed.argument_types = arguments.into_iter().map(str::to_owned).collect();
        if changed.argument_types.len() != 1 {
            assert!(validate_shape(&changed).is_err());
        }
    }
}

#[test]
fn scalar_constructor_requires_all_empty_cases_and_historical_effect_flags() {
    let mut scalar = fixture();
    scalar.argument_types = vec!["u32".into(), "u32".into()];
    for mutation in 0..12 {
        let mut changed = scalar.clone();
        match mutation {
            0 => changed.mutates_arguments = true,
            1 => changed.memory_effects = true,
            2 => changed.may_panic = true,
            3 => changed.always_returns = false,
            4 => changed.reference_outputs = Some(vec![]),
            5 => changed.component_outputs = Some(vec![]),
            6 => changed.memory_reads = Some(vec![]),
            7 => changed.tagged_output.as_mut().unwrap().input_cases.clear(),
            8 => changed.tagged_output.as_mut().unwrap().output_cases.clear(),
            9 => changed.tagged_output.as_mut().unwrap().alternatives.clear(),
            10 => {
                changed.tagged_output.as_mut().unwrap().alternatives[0].condition =
                    Condition::Boolean {
                        condition: crate::HirRef {
                            krate: 0,
                            owner: 1,
                            local: 2,
                        },
                        outcome: true,
                    }
            }
            _ => changed.tagged_output.as_mut().unwrap().output_cases[0]
                .components
                .push(vec![]),
        }
        assert!(validate_shape(&changed).is_err(), "mutation {mutation}");
    }
}
