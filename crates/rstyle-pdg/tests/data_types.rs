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
        r#"{"kind":"array","value":{"element":{"kind":"scalar","value":"u8"},"length":18446744073709551616}}"#,
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
fn read_only_call_registration_keeps_data_v1_distinct() {
    use rstyle_pdg::extraction_profile::ExtractionProfile;
    let profile = ExtractionProfile::select("hir-data-v2").unwrap();
    assert_eq!(profile, ExtractionProfile::DataV2);
    assert_eq!(profile.policy().id, "rstyle.rust.hir-data");
    assert_eq!(profile.policy().version, "2.0.0");
    assert_eq!(
        profile.data_matching(),
        Some("rstyle.ccgraph.data-calls/1.0.0")
    );
    assert_eq!(
        ExtractionProfile::DataV1.data_matching(),
        Some("rstyle.ccgraph.data/1.0.0")
    );
    assert_eq!(profile.capabilities().len(), 11);
    assert_eq!(ExtractionProfile::DataV1.capabilities().len(), 10);
    assert!(!ExtractionProfile::DataV1.supports("rust.verified-read-only-helper-calls"));
    assert!(profile.supports("rust.verified-read-only-helper-calls"));
    assert!(rstyle_pdg::extraction_profile::registered(
        &profile.policy()
    ));
}

#[test]
fn inherent_registration_keeps_earlier_data_contracts_distinct() {
    use rstyle_pdg::extraction_profile::ExtractionProfile;
    let profile = ExtractionProfile::select("hir-data-v3").unwrap();
    assert_eq!(profile, ExtractionProfile::DataV3);
    assert_eq!(profile.policy().version, "3.53.0");
    assert_eq!(
        profile.data_matching(),
        Some("rstyle.ccgraph.data-calls/1.0.0")
    );
    assert_eq!(profile.capabilities().len(), 68);
    assert!(profile.supports("rust.verified-inherent-helper-calls"));
    for earlier in [ExtractionProfile::DataV1, ExtractionProfile::DataV2] {
        assert!(!earlier.supports("rust.verified-inherent-helper-calls"));
        assert_ne!(earlier.policy(), profile.policy());
    }
    assert!(rstyle_pdg::extraction_profile::registered(
        &profile.policy()
    ));
}

#[test]
fn token_serialization_cannot_turn_a_resource_failure_into_observational_success() {
    let tree = TypeTree::Tuple(vec![TypeTree::Scalar("u8".into()); 3000]);
    rstyle_pdg::concrete_types::validate(&tree).unwrap();
    let error = data_types::token(&tree).unwrap_err();
    assert!(error.contains("byte budget"), "{error}");
}

#[test]
fn nominal_tokens_preserve_namespace_fields_and_native_width_requirements() {
    let expected = r#"{"kind":"struct","value":{"nominal":{"source_identity":"package-a","package_name":"api","package_version":"1.0.0","manifest_digest":"manifest","definition_path":"types::Api","declaration_digest":"declaration"},"fields":[["length",{"kind":"scalar","value":"usize"}],["flag",{"kind":"scalar","value":"bool"}]]}}"#;
    let token = data_types::parse(expected).unwrap();
    assert_eq!(token.text().unwrap(), expected);
    assert!(token.native_width());
    assert_ne!(
        token,
        data_types::parse(&expected.replace("package-a", "package-b")).unwrap()
    );
    assert!(
        !data_types::parse(&expected.replace("usize", "u64"))
            .unwrap()
            .native_width()
    );
    assert!(
        data_types::parse(&expected.replace("\"source_identity\":\"package-a\",", "")).is_err()
    );
    assert!(data_types::parse(&expected.replace("package-a", "")).is_err());
    assert!(data_types::parse(&expected.replace("\"flag\"", "\"length\"")).is_err());
}
