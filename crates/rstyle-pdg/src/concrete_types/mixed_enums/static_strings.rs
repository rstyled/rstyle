//! Source observations remain separate from guarded value permission.
use super::*;

fn nominal(name: &str, source: &crate::SourceText) -> Nominal {
    Nominal {
        source_identity: Some("selected".into()),
        package_name: "sample".into(),
        package_version: "1".into(),
        manifest_digest: crate::source_identity::checksum(b"manifest"),
        definition_path: name.into(),
        declaration_digest: source.checksum().into(),
    }
}

fn observed() -> TypeTree {
    let source: crate::SourceText =
        "struct Payload { name: &'static str, conditional: bool }".into();
    let payload = TypeTree::Struct {
        nominal: nominal("Payload", &source),
        source,
        fields: vec![
            Field {
                position: 0,
                name: "name".into(),
                declaration: "&'static str".into(),
                ty: TypeTree::Shared(Box::new(TypeTree::Str)),
            },
            Field {
                position: 1,
                name: "conditional".into(),
                declaration: "bool".into(),
                ty: TypeTree::Scalar("bool".into()),
            },
        ],
    };
    let source: crate::SourceText = "enum Checked { Empty, Value(Payload) }".into();
    TypeTree::MixedEnum {
        nominal: nominal("Checked", &source),
        source,
        variants: vec![
            MixedVariant {
                position: 0,
                name: "Empty".into(),
                shape: VariantShape::Unit {},
            },
            MixedVariant {
                position: 1,
                name: "Value".into(),
                shape: VariantShape::SingleTuple {
                    field: Field {
                        position: 0,
                        name: "0".into(),
                        declaration: "Payload".into(),
                        ty: payload,
                    },
                },
            },
        ],
    }
}

#[test]
fn complete_reference_payload_observations_preserve_but_do_not_admit_values() {
    let tree = observed();
    validate(&tree).unwrap();
    let token = crate::data_types::TypeToken::from_tree(&tree);
    let text = crate::data_types::token(&tree).unwrap();
    assert_eq!(crate::data_types::parse(&text).unwrap(), token);
    assert!(crate::data_origins::shared(&token));
    for profile in [
        crate::extraction_profile::ExtractionProfile::DataV2,
        crate::extraction_profile::ExtractionProfile::DataV3,
    ] {
        assert!(crate::data_types::parse_for(&text, profile).is_err());
    }
}

#[test]
fn broader_reference_storage_is_not_part_of_the_closed_payload_grammar() {
    for referent in [
        TypeTree::Scalar("u8".into()),
        TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into()))),
        TypeTree::Shared(Box::new(TypeTree::Str)),
    ] {
        let tree = TypeTree::Shared(Box::new(referent));
        assert!(!closed_payload(&tree));
        assert!(!closed_payload(&crate::data_types::TypeToken::from_tree(
            &tree
        )));
    }
}

#[test]
fn reference_payload_capture_keeps_exact_nested_field_correspondence() {
    for mutation in 0..3 {
        let mut tree = observed();
        let TypeTree::MixedEnum { variants, .. } = &mut tree else {
            unreachable!();
        };
        let VariantShape::SingleTuple { field } = &mut variants[1].shape else {
            unreachable!();
        };
        let TypeTree::Struct { fields, .. } = &mut field.ty else {
            unreachable!();
        };
        match mutation {
            0 => fields[0].declaration = "&str".into(),
            1 => fields[0].ty = TypeTree::Shared(Box::new(TypeTree::Scalar("u8".into()))),
            _ => fields.swap(0, 1),
        }
        assert!(validate(&tree).is_err(), "mutation {mutation}");
    }
}

#[test]
fn inverse_result_observes_the_original_nominal_empty_error_without_value_permission() {
    let TypeTree::MixedEnum { variants, .. } = observed() else {
        unreachable!();
    };
    let VariantShape::SingleTuple { field } = &variants[1].shape else {
        unreachable!();
    };
    let source: crate::SourceText = "struct DifferentVariant;".into();
    let result = TypeTree::Result {
        ok: Box::new(field.ty.clone()),
        err: Box::new(TypeTree::Struct {
            nominal: nominal("DifferentVariant", &source),
            source,
            fields: vec![],
        }),
    };
    validate(&result).unwrap();
    let text = crate::data_types::token(&result).unwrap();
    assert_eq!(
        crate::data_types::parse(&text).unwrap(),
        crate::data_types::TypeToken::from_tree(&result)
    );
    for profile in [
        crate::extraction_profile::ExtractionProfile::DataV2,
        crate::extraction_profile::ExtractionProfile::DataV3,
    ] {
        assert!(crate::data_types::parse_for(&text, profile).is_err());
    }
}
