use super::*;

fn shared() -> TypeToken {
    TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        "u8".into(),
    )))))
}

fn product() -> TypeToken {
    TypeToken::Tuple(vec![
        shared(),
        TypeToken::Tuple(vec![TypeToken::Scalar("usize".into()), shared()]),
    ])
}

fn outputs() -> Vec<Output> {
    let mut outputs = roster::leaves(&product()).unwrap();
    for (position, output) in outputs.iter_mut().enumerate() {
        output.origins.push(Origin {
            parameter: position as u32,
            path: vec![Projection::SharedReferent],
        });
    }
    outputs
}

#[test]
fn typed_roster_preserves_distinct_nested_reference_leaves() {
    let outputs = outputs();
    roster::verify(&product(), &outputs).unwrap();
    assert_eq!(
        outputs
            .iter()
            .map(|output| output.selectors.clone())
            .collect::<Vec<_>>(),
        vec![vec![0], vec![1, 1]]
    );
    assert_eq!(outputs[0].origins[0].parameter, 0);
    assert_eq!(outputs[1].origins[0].parameter, 1);
}

#[test]
fn leaf_roster_rejects_omissions_duplicates_reordering_and_wrong_types() {
    let mut omitted = outputs();
    omitted.pop();
    assert!(roster::verify(&product(), &omitted).is_err());
    let mut duplicated = outputs();
    duplicated[1] = duplicated[0].clone();
    assert!(roster::verify(&product(), &duplicated).is_err());
    let mut reordered = outputs();
    reordered.swap(0, 1);
    assert!(roster::verify(&product(), &reordered).is_err());
    let mut wrong_type = outputs();
    wrong_type[0].ty = TypeToken::Shared(Box::new(TypeToken::Str));
    assert!(roster::verify(&product(), &wrong_type).is_err());
}

#[test]
fn missing_origins_and_unsupported_container_shapes_are_not_empty_evidence() {
    let mut missing = outputs();
    missing[1].origins.clear();
    assert!(roster::verify(&product(), &missing).is_err());
    let array = TypeToken::Tuple(vec![TypeToken::Array {
        element: Box::new(shared()),
        length: 2,
    }]);
    assert!(roster::leaves(&array).is_err());
    assert!(
        roster::leaves(&TypeToken::Tuple(vec![TypeToken::Option(Box::new(
            shared()
        ))]))
        .is_err()
    );
}

#[test]
fn selectors_cannot_address_scalar_missing_or_reference_interior_components() {
    let outputs = outputs();
    assert_eq!(roster::selected(&outputs, &[]).unwrap().len(), 2);
    assert_eq!(
        roster::selected(&outputs, &[1]).unwrap()[0].selectors,
        vec![1, 1]
    );
    for invalid in [vec![1, 0], vec![2], vec![0, 0], vec![1, 1, 0]] {
        assert!(roster::selected(&outputs, &invalid).is_err(), "{invalid:?}");
    }
}
