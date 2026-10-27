//! Read effects and panic outcomes must survive alongside tagged reference cases.
use super::*;
use crate::{
    copy_tagged_values::origins::{Case, CaseKind},
    data_types::TypeToken,
    field_locations::Projection,
    helper_calls::Summary,
};

fn contract() -> Summary {
    Summary {
        identity: SUMMARY.into(),
        argument_types: vec!["shared-input".into(), "static-message".into()],
        result_type: "tagged-result".into(),
        mutates_arguments: false,
        memory_effects: true,
        may_panic: true,
        always_returns: false,
        argument_evaluation: "source-order-once/1".into(),
        result_dependence: super::super::tagged_conversions::RESULT_DEPENDENCE.into(),
        bounded_loop: None,
        reference_outputs: None,
        component_outputs: None,
        memory_reads: Some(vec![crate::helper_reads::Read {
            operation: "actual-byte-read".into(),
            access: 0,
            parameter: 0,
            path: vec![Projection::SharedReferent, Projection::CheckedElement],
            value: TypeToken::Scalar("u8".into()),
        }]),
        tagged_output: Some(WireContract {
            input_cases: vec![],
            output_cases: vec![Case {
                kind: CaseKind::ResultOk,
                components: vec![],
            }],
            alternatives: vec![],
        }),
    }
}

#[test]
fn guarded_outputs_do_not_erase_memory_or_possible_panic() {
    let summary = contract();
    validate_shape(&summary).unwrap();
    let replay: Summary = serde_json::from_value(serde_json::to_value(&summary).unwrap()).unwrap();
    assert!(replay == summary);
    assert!(replay.may_panic && !replay.always_returns);
    assert_eq!(
        replay.memory_reads.unwrap()[0].operation,
        "actual-byte-read"
    );
}

#[test]
fn incomplete_contract_is_not_a_memory_free_or_total_summary() {
    for mutation in 0..10 {
        let mut summary = contract();
        match mutation {
            0 => summary.mutates_arguments = true,
            1 => summary.memory_effects = false,
            2 => summary.memory_reads = None,
            3 => summary.memory_reads = Some(vec![]),
            4 => summary.always_returns = true,
            5 => summary.may_panic = false,
            6 => summary.tagged_output = None,
            7 => summary.reference_outputs = Some(vec![]),
            8 => summary.component_outputs = Some(vec![]),
            _ => summary.argument_evaluation = "repeated".into(),
        }
        assert!(validate_shape(&summary).is_err(), "mutation {mutation}");
    }
}

#[test]
fn historical_tagged_contracts_cannot_acquire_read_or_panic_permission() {
    let summary = contract();
    for identity in [
        super::super::tagged_conversions::SUMMARY,
        super::super::named_errors::SUMMARY,
        super::super::slice_lengths::SUMMARY,
    ] {
        let mut changed = summary.clone();
        changed.identity = identity.into();
        changed.argument_types = vec!["static-message".into()];
        assert!(super::super::tagged_conversions::validate_shape_as(&changed, identity).is_err());
    }
}
