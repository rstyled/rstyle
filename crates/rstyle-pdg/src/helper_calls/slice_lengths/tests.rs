//! Transport adversaries do not substitute for independently verified callee graphs.
use super::*;
use crate::copy_tagged_values::origins::{Alternative, Case, CaseKind, Condition};

fn fixture() -> super::super::Summary {
    let facts: crate::HirFnFacts =
        serde_json::from_str(include_str!("../../copy_tagged_values/tests/0.json")).unwrap();
    let contract = WireContract {
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![],
        }],
        output_cases: vec![Case {
            kind: CaseKind::ResultOk,
            components: vec![],
        }],
        alternatives: vec![Alternative {
            input: CaseKind::Whole,
            output: CaseKind::ResultOk,
            condition: Condition::Boolean {
                condition: facts.body.hir.clone(),
                outcome: false,
            },
            leaves: vec![],
        }],
    };
    let mut summary = super::super::tagged_conversions::summary(&facts, contract).unwrap();
    summary.identity = SUMMARY.into();
    summary.argument_types = vec!["&[u8]".into(), "usize".into(), "&str".into()];
    summary
}

#[test]
fn checked_length_transport_keeps_boolean_scope_and_distinct_summary_identity() {
    let summary = fixture();
    assert!(validate_shape(&summary).is_ok());
    assert!(super::super::tagged_conversions::validate_shape(&summary).is_err());
    assert!(super::super::named_errors::validate_shape(&summary).is_err());
    let encoded = serde_json::to_value(&summary).unwrap();
    assert_eq!(
        encoded["tagged_output"]["alternatives"][0]["condition"]["kind"],
        "boolean"
    );
    assert_eq!(
        encoded["tagged_output"]["alternatives"][0]["condition"]["value"]["outcome"],
        false
    );
    let decoded: super::super::Summary = serde_json::from_value(encoded).unwrap();
    assert!(summary == decoded);
}

#[test]
fn checked_length_cannot_claim_effects_missing_origins_or_another_arity() {
    for mutation in 0..14 {
        let mut summary = fixture();
        match mutation {
            0 => summary.mutates_arguments = true,
            1 => summary.memory_effects = true,
            2 => summary.may_panic = true,
            3 => summary.always_returns = false,
            4 => summary.reference_outputs = Some(vec![]),
            5 => summary.component_outputs = Some(vec![]),
            6 => summary.memory_reads = Some(vec![]),
            7 => summary.tagged_output = None,
            8 => summary.argument_types.truncate(1),
            9 => summary.argument_types.push("usize".into()),
            10 => summary.argument_types[1].clear(),
            11 => summary.argument_evaluation = "unordered".into(),
            12 => summary.result_dependence = "all-inputs".into(),
            _ => summary.identity = super::super::tagged_conversions::SUMMARY.into(),
        }
        assert!(validate_shape(&summary).is_err(), "mutation {mutation}");
    }
}
