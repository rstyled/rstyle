//! A displayed Range name cannot replace the actual local captured interface.
use super::*;
use crate::product_instances::test_support::conversion as fixture;

fn range() -> TypeTree {
    let fixture: Value = serde_json::from_str(include_str!("../model/range/fixture.json")).unwrap();
    serde_json::from_value(serde_json::json!({"kind": "RangeUsize", "evidence": fixture["range"]}))
        .unwrap()
}

fn interface(input: TypeTree, result: TypeTree) -> InterfaceObservations {
    InterfaceObservations {
        identity: crate::concrete_types::IDENTITY.into(),
        parameters: vec![TypeObservation::Captured { ty: input }],
        result: TypeObservation::Captured { ty: result },
    }
}

#[test]
fn selected_from_requires_owned_range_input_and_complete_equal_span_result() {
    let (mut local, _, span) = fixture();
    local
        .build
        .frontend_options
        .insert_data(interface(span.clone(), span.clone()));
    assert!(local_input(&local, &span).is_err());
    local
        .build
        .frontend_options
        .insert_data(interface(range(), TypeTree::Scalar("usize".into())));
    assert!(local_input(&local, &span).is_err());
    local
        .build
        .frontend_options
        .insert_data(interface(range().wrapped(TypeTree::Shared), span.clone()));
    assert!(local_input(&local, &span).is_err());
}

#[test]
fn selected_from_cannot_replace_actual_source_by_a_range_observation() {
    let (mut unrelated, _, span) = fixture();
    unrelated
        .build
        .frontend_options
        .insert_data(interface(range(), span.clone()));
    assert!(local_input(&unrelated, &span).is_err());
    let mut missing = unrelated.clone();
    missing
        .build
        .frontend_options
        .remove(crate::concrete_types::KEY);
    assert!(local_input(&missing, &span).is_err());
}
