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
        "f16",
        "f128",
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
    assert_eq!(profile.policy().version, "3.128.0");
    assert_eq!(
        profile.data_matching(),
        Some("rstyle.ccgraph.data-calls/1.0.0")
    );
    assert_eq!(profile.capabilities().len(), 156);
    assert!(profile.supports("rust.pinned-cleanup-macro-imports"));
    assert!(profile.supports("rust.named-error-reader-flow"));
    let mut before_readers = profile.policy();
    before_readers.version = "3.120.0".into();
    assert!(!rstyle_pdg::extraction_profile::policy_supports(
        &before_readers,
        "rust.named-error-reader-flow"
    ));
    let mut before_library_routing = profile.policy();
    before_library_routing.version = "3.119.0".into();
    assert!(!rstyle_pdg::extraction_profile::policy_supports(
        &before_library_routing,
        "rust.pinned-cleanup-macro-imports"
    ));
    assert!(profile.supports("rust.checked-scalar-instances"));
    assert!(profile.supports("rust.component-helper-results"));
    let mut previous = profile.policy();
    previous.version = "3.83.0".into();
    assert!(!rstyle_pdg::extraction_profile::policy_supports(
        &previous,
        "rust.component-helper-results"
    ));
    assert!(profile.supports("rust.verified-inherent-helper-calls"));
    assert!(profile.supports("rust.verified-cross-unit-scalar-helpers"));
    assert!(profile.supports("rust.explicit-u8-fieldless-enums"));
    for earlier in [ExtractionProfile::DataV1, ExtractionProfile::DataV2] {
        assert!(!earlier.supports("rust.checked-scalar-instances"));
        assert!(!earlier.supports("rust.component-helper-results"));
        assert!(!earlier.supports("rust.verified-inherent-helper-calls"));
        assert!(!earlier.supports("rust.verified-cross-unit-scalar-helpers"));
        assert!(!earlier.supports("rust.explicit-u8-fieldless-enums"));
        assert_ne!(earlier.policy(), profile.policy());
    }
    assert!(rstyle_pdg::extraction_profile::registered(
        &profile.policy()
    ));
}

#[test]
fn tagged_conversion_authority_does_not_extend_historical_policies() {
    use rstyle_pdg::extraction_profile::{ExtractionProfile, policy_supports};
    let capability = "rust.copy-tagged-conversion-values";
    let current = ExtractionProfile::DataV3.policy();
    assert!(policy_supports(&current, capability));

    let mut historical = current;
    historical.version = "3.102.0".into();
    assert!(!policy_supports(&historical, capability));
    for earlier in [ExtractionProfile::DataV1, ExtractionProfile::DataV2] {
        assert!(!earlier.supports(capability));
    }
}

#[test]
fn string_byte_view_authority_is_new_and_keeps_the_existing_byte_contract() {
    use rstyle_pdg::extraction_profile::{ExtractionProfile, policy_supports};
    let current = ExtractionProfile::DataV3.policy();
    assert!(policy_supports(
        &current,
        "rust.checked-string-byte-view-instances"
    ));
    assert!(policy_supports(
        &current,
        "rust.checked-reference-constructor-instances"
    ));
    let mut historical = current;
    historical.version = "3.85.0".into();
    assert!(!policy_supports(
        &historical,
        "rust.checked-string-byte-view-instances"
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
fn callback_observations_do_not_extend_the_previous_document_policy() {
    use rstyle_pdg::extraction_profile::{ExtractionProfile, policy_supports};
    let profile = ExtractionProfile::DataV3;
    let capability = "rust.usize-reader-callback-observations";
    assert!(profile.supports(capability));
    assert!(policy_supports(&profile.policy(), capability));
    let mut previous = profile.policy();
    previous.version = "3.125.0".into();

    assert!(!policy_supports(&previous, capability));
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

#[test]
fn character_vocabulary_preserves_a_distinct_standalone_domain() {
    let character = TypeTree::Scalar("char".into());
    assert!(!rstyle_pdg::concrete_types::scalar("char"));
    assert_eq!(data_types::token(&character).unwrap(), "char");
    assert_eq!(
        data_types::parse("char").unwrap(),
        TypeToken::Scalar("char".into())
    );
    for structured in [
        TypeTree::Shared(Box::new(character.clone())),
        TypeTree::Slice(Box::new(character.clone())),
        TypeTree::Tuple(vec![character.clone()]),
        TypeTree::Array {
            element: Box::new(character.clone()),
            length: 1,
        },
        TypeTree::Option(Box::new(character)),
    ] {
        assert!(rstyle_pdg::concrete_types::validate(&structured).is_err());
        let serialized = serde_json::to_string(&TypeToken::from_tree(&structured)).unwrap();
        assert!(data_types::parse(&serialized).is_err(), "{serialized}");
    }
}

#[test]
fn float_vocabulary_requires_standalone_widths_and_current_policy() {
    use rstyle_pdg::extraction_profile::ExtractionProfile;
    for width in ["f32", "f64"] {
        let tree = TypeTree::Scalar(width.into());
        assert!(!rstyle_pdg::concrete_types::scalar(width));
        assert_eq!(data_types::token(&tree).unwrap(), width);
        assert_eq!(
            data_types::parse(width).unwrap(),
            TypeToken::Scalar(width.into())
        );
        for earlier in [ExtractionProfile::DataV1, ExtractionProfile::DataV2] {
            assert!(!earlier.supports("rust.float-kernel-scalars"));
        }
        for structured in [
            TypeTree::Shared(Box::new(tree.clone())),
            TypeTree::Slice(Box::new(tree.clone())),
            TypeTree::Tuple(vec![tree.clone()]),
            TypeTree::Array {
                element: Box::new(tree.clone()),
                length: 1,
            },
            TypeTree::Option(Box::new(tree)),
        ] {
            assert!(rstyle_pdg::concrete_types::validate(&structured).is_err());
            let serialized = serde_json::to_string(&TypeToken::from_tree(&structured)).unwrap();
            assert!(data_types::parse(&serialized).is_err(), "{serialized}");
        }
    }
}

#[test]
fn original_from_authority_requires_its_new_document_policy() {
    use rstyle_pdg::extraction_profile::{ExtractionProfile, policy_supports};
    let current = ExtractionProfile::DataV3.policy();
    assert!(policy_supports(
        &current,
        "rust.checked-reference-forwarder-instances"
    ));
    let mut previous = current;
    previous.version = "3.86.0".into();
    assert!(!policy_supports(
        &previous,
        "rust.checked-reference-forwarder-instances"
    ));
    for profile in [ExtractionProfile::DataV1, ExtractionProfile::DataV2] {
        assert!(!profile.supports("rust.checked-reference-forwarder-instances"));
    }
}

#[test]
fn tagged_helper_authority_does_not_extend_value_only_policy() {
    use rstyle_pdg::extraction_profile::{ExtractionProfile, policy_supports};
    let current = ExtractionProfile::DataV3.policy();
    let capability = "rust.copy-tagged-conversion-helper-calls";
    assert!(policy_supports(&current, capability));
    let mut historical = current;
    historical.version = "3.103.0".into();
    assert!(!policy_supports(&historical, capability));
}

#[test]
fn slice_prefix_observations_do_not_extend_historical_policies() {
    use rstyle_pdg::extraction_profile::{ExtractionProfile, policy_supports};
    let current = ExtractionProfile::DataV3.policy();
    let capability = "rust.slice-prefix-product-results";
    assert!(policy_supports(&current, capability));
    let mut historical = current;
    historical.version = "3.104.0".into();
    assert!(!policy_supports(&historical, capability));
    for profile in [
        ExtractionProfile::R0,
        ExtractionProfile::DataV1,
        ExtractionProfile::DataV2,
    ] {
        assert!(!profile.supports(capability));
    }
}

#[test]
fn owned_error_transfer_does_not_extend_historical_policies() {
    let profile = rstyle_pdg::extraction_profile::ExtractionProfile::DataV3;
    assert!(profile.supports("rust.named-error-owned-transfer"));
    let mut before_owned_errors = profile.policy();
    before_owned_errors.version = "3.123.0".into();
    assert!(!rstyle_pdg::extraction_profile::policy_supports(
        &before_owned_errors,
        "rust.named-error-owned-transfer"
    ));
}
