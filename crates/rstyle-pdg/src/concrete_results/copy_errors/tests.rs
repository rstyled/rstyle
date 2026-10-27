use super::*;

fn original_error() -> TypeToken {
    // Reuse the actual Error observation from the original 141-definition raw
    // capture. These are existing source identities, not newly minted seals.
    serde_json::from_str(r#"{"kind":"fieldless_enum","value":{"nominal":{"source_identity":"path+file:///home/dev/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/base64ct-1.7.3#base64ct@1.7.3","package_name":"base64ct","package_version":"1.7.3","manifest_digest":"sha256:e1d351e3b0da5410a327430a53dc4ac81b59f798bd3981c1611f4a89e7305497","definition_path":"errors::Error","declaration_digest":"sha256:1662d5be3fb8c050b19eb421feab9ccc14fb1a8d3902a21d26319472d94d9152"},"variants":[["InvalidEncoding",0],["InvalidLength",1]]}}"#).unwrap()
}

#[test]
fn actual_error_stays_fieldless_and_has_only_the_selected_success_shapes() {
    let original = original_error();
    let nominal = error(&original).unwrap();
    assert_eq!(nominal.definition_path, "errors::Error");
    assert!(crate::borrowed_result_types::error(&original).is_none());
    let bytes = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        "u8".into(),
    )))));
    for ok in [TypeToken::Scalar("usize".into()), bytes.clone()] {
        assert!(GRAMMAR.permits(&ok, &original));
    }
    for ok in [
        TypeToken::Scalar("unit".into()),
        TypeToken::Scalar("u32".into()),
        TypeToken::Scalar("bool".into()),
        TypeToken::Tuple(vec![TypeToken::Scalar("usize".into()), bytes.clone()]),
        TypeToken::Option(Box::new(bytes)),
    ] {
        assert!(!GRAMMAR.permits(&ok, &original));
    }
}

#[test]
fn error_shape_does_not_promote_mixed_or_ambiguous_enum_observations() {
    let original = original_error();
    let TypeToken::FieldlessEnum { nominal, .. } = &original else {
        unreachable!()
    };
    assert!(
        error(&TypeToken::MixedEnum {
            nominal: nominal.clone(),
            variants: vec![(
                "InvalidLength".into(),
                crate::concrete_types::VariantShape::Unit {}
            )],
        })
        .is_none()
    );
    for variants in [
        vec![],
        vec![("".into(), 0)],
        vec![("InvalidLength".into(), 0), ("InvalidLength".into(), 1)],
        vec![("InvalidEncoding".into(), 1), ("InvalidLength".into(), 1)],
    ] {
        assert!(
            error(&TypeToken::FieldlessEnum {
                nominal: nominal.clone(),
                variants,
            })
            .is_none()
        );
    }
    let mut missing_source = original;
    let TypeToken::FieldlessEnum { nominal, .. } = &mut missing_source else {
        unreachable!()
    };
    nominal.source_identity = None;
    assert!(error(&missing_source).is_none());
}

#[test]
fn direct_and_hidden_copy_results_require_current_parser_and_document_policies() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    let direct = TypeToken::Result {
        ok: Box::new(TypeToken::Scalar("usize".into())),
        err: Box::new(original_error()),
    };
    let hidden = TypeToken::Shared(Box::new(TypeToken::Tuple(vec![direct.clone()])));
    let historical = crate::ProfileId {
        id: DataV3.policy().id,
        version: "3.87.0".into(),
    };
    assert!(!crate::extraction_profile::policy_supports(
        &historical,
        CAPABILITY
    ));
    for token in [direct, hidden] {
        assert!(contains_copy_result(&token));
        let text = token.text().unwrap();
        assert!(crate::data_types::parse_for(&text, DataV3).is_ok());
        assert!(crate::data_types::parse_for(&text, DataV2).is_err());
        assert!(
            crate::extraction_profile::require_document(
                &historical,
                CAPABILITY,
                || contains_copy_result(&token),
                "Copy Result historical policy is closed",
            )
            .is_err()
        );
    }
    assert!(!contains_copy_result(&original_error()));
    assert!(!contains_copy_result(&TypeToken::Scalar("usize".into())));
}
