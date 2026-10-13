use rstyle_pdg::{
    concrete_types::TypeTree,
    data_types::{self, TypeToken},
};

#[test]
fn concrete_data_interface_vocabulary_is_exact_and_bounded() {
    let tree = TypeTree::Shared(Box::new(TypeTree::Tuple(vec![
        TypeTree::Scalar("i32".into()),
        TypeTree::Scalar("usize".into()),
    ])));
    let expected = r#"{"kind":"shared","value":{"kind":"tuple","value":[{"kind":"scalar","value":"i32"},{"kind":"scalar","value":"usize"}]}}"#;
    assert_eq!(data_types::token(&tree).unwrap(), expected);
    assert!(data_types::parse(expected).unwrap().native_width());
    assert_eq!(
        data_types::parse("i32").unwrap(),
        TypeToken::Scalar("i32".into())
    );
    for token in [
        r#"{"kind":"shared", "value":{"kind":"scalar","value":"i32"}}"#,
        r#"{"kind":"mutable","value":{"kind":"scalar","value":"i32"}}"#,
        r#"{"kind":"array","value":{"element":{"kind":"scalar","value":"u8"},"length":4097}}"#,
        r#"{"kind":"scalar","value":"i32"}"#,
        "f32",
        "&i32",
    ] {
        assert!(data_types::parse(token).is_err(), "{token}");
    }
}

#[test]
fn local_data_registration_preserves_historical_capability_sets() {
    use rstyle_pdg::extraction_profile::ExtractionProfile;
    assert_eq!(ExtractionProfile::HelpersV1.capabilities().len(), 9);
    assert_eq!(ExtractionProfile::CallsV1.capabilities().len(), 8);
    assert_eq!(ExtractionProfile::R0.capabilities().len(), 0);
    assert!(!ExtractionProfile::HelpersV1.supports("rust.concrete-read-only-data"));
    assert_eq!(
        ExtractionProfile::select("hir-data-v1").unwrap(),
        ExtractionProfile::DataV1
    );
    assert_eq!(
        ExtractionProfile::DataV1.policy().id,
        "rstyle.rust.hir-data"
    );
    assert_eq!(ExtractionProfile::DataV1.policy().version, "1.0.0");
}

#[test]
fn token_serialization_cannot_turn_a_resource_failure_into_observational_success() {
    let tree = TypeTree::Tuple(vec![TypeTree::Scalar("u8".into()); 3000]);
    rstyle_pdg::concrete_types::validate(&tree).unwrap();
    let error = data_types::token(&tree).unwrap_err();
    assert!(error.contains("byte budget"), "{error}");
}
