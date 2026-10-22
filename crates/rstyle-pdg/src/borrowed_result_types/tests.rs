//! Independent source classifications, deliberately before capability activation.
use super::*;
use crate::concrete_types::{Field, MixedVariant};

fn original_error() -> TypeTree {
    let selection: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../eval/coverage-development/uncompressed-size/selection.json"
    ))
    .unwrap();
    let source: crate::SourceText = selection["owners"][0]["source"].as_str().unwrap().into();
    assert_eq!(
        source.identity(),
        "sha256:0af2d4843e7e77492028bac9e0b32176deda0fe8fd79c980016961c0df5b3569"
    );
    let nominal = Nominal {
        source_identity: Some("lz4_flex@0.11.6".into()),
        package_name: "lz4_flex".into(),
        package_version: "0.11.6".into(),
        manifest_digest: "sha256:1a097ce53b65d7c6164b9c67ef341144b398b4ce516ec875cecb07e71bd5afea"
            .into(),
        definition_path: "lz4_flex::block::DecompressError".into(),
        declaration_digest: source.identity().into(),
    };
    let variants = [
        "OutputTooSmall",
        "LiteralOutOfBounds",
        "ExpectedAnotherByte",
        "OffsetZero",
        "OffsetOutOfBounds",
    ]
    .into_iter()
    .enumerate()
    .map(|(position, name)| MixedVariant {
        position,
        name: name.into(),
        shape: if position == 0 {
            named_fields()
        } else {
            VariantShape::Unit {}
        },
    })
    .collect();
    TypeTree::MixedEnum {
        nominal,
        source,
        variants,
    }
}

fn named_fields() -> VariantShape<Field> {
    VariantShape::Named {
        fields: ["expected", "actual"]
            .into_iter()
            .enumerate()
            .map(|(position, name)| {
                (
                    name.into(),
                    Field {
                        position,
                        name: name.into(),
                        declaration: "usize".into(),
                        ty: TypeTree::Scalar("usize".into()),
                    },
                )
            })
            .collect(),
    }
}

fn byte_slice() -> TypeTree {
    TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
        "u8".into(),
    )))))
}

fn successes() -> [TypeTree; 2] {
    [
        byte_slice(),
        TypeTree::Tuple(vec![TypeTree::Scalar("usize".into()), byte_slice()]),
    ]
}

#[test]
fn selected_original_results_are_complete_observations_without_copy_permission() {
    let error = original_error();
    crate::concrete_types::validate(&error).unwrap();
    let TypeTree::MixedEnum {
        source, variants, ..
    } = &error
    else {
        unreachable!()
    };
    assert_eq!(variants.len(), 5);
    let syntax: syn::ItemEnum = syn::parse_str(source).unwrap();
    assert!(
        syntax
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("non_exhaustive"))
    );
    assert!(!source.contains("Copy"));
    for ok in successes() {
        assert!(tree_result(&ok, &error));
        let tree = TypeTree::Result {
            ok: Box::new(ok),
            err: Box::new(error.clone()),
        };
        crate::concrete_types::validate(&tree).unwrap();
        let encoded = serde_json::to_vec(&tree).unwrap();
        assert_eq!(serde_json::from_slice::<TypeTree>(&encoded).unwrap(), tree);
        let token = TypeToken::from_tree(&tree);
        let text = token.text().unwrap();
        assert_eq!(crate::data_types::parse(&text).unwrap(), token);
        assert!(uses_result(&token));
        assert!(owned_use(&token));
    }
}

#[test]
fn original_error_source_field_inventory_and_type_tampering_reject() {
    for mutation in 0..8 {
        let mut tree = original_error();
        let TypeTree::MixedEnum {
            variants, source, ..
        } = &mut tree
        else {
            unreachable!()
        };
        if mutation == 0 {
            variants.pop();
        } else if mutation == 1 {
            *source = source.replace("OffsetZero", "DifferentZero").into();
        } else {
            let VariantShape::Named { fields } = &mut variants[0].shape else {
                unreachable!()
            };
            match mutation {
                2 => fields.swap(0, 1),
                3 => fields[0].0 = "actual".into(),
                4 => fields[0].1.ty = TypeTree::Scalar("unit".into()),
                5 => fields[0].1.ty = TypeTree::Scalar("u64".into()),
                6 => fields[0].1.position = 1,
                _ => fields[0].1.declaration = "u64".into(),
            }
        }
        assert!(
            crate::concrete_types::validate(&tree).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn success_contract_rejects_other_scalar_order_and_reference_shapes() {
    let error = original_error();
    for ok in [
        TypeTree::Scalar("unit".into()),
        TypeTree::Scalar("usize".into()),
        TypeTree::Tuple(vec![byte_slice(), TypeTree::Scalar("usize".into())]),
        TypeTree::Tuple(vec![TypeTree::Scalar("u64".into()), byte_slice()]),
        TypeTree::Shared(Box::new(TypeTree::Scalar("u8".into()))),
        TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into()))),
    ] {
        assert!(!tree_result(&ok, &error));
        assert!(!result(
            &TypeToken::from_tree(&ok),
            &TypeToken::from_tree(&error)
        ));
    }
}

#[test]
fn error_contract_rejects_reference_owner_and_nested_result_fields() {
    let original = TypeToken::from_tree(&original_error());
    let TypeToken::MixedEnum { nominal, .. } = &original else {
        unreachable!()
    };
    let owner = TypeToken::Struct {
        nominal: nominal.clone(),
        fields: vec![("value".into(), TypeToken::Scalar("usize".into()))],
    };
    for field in [
        TypeToken::Shared(Box::new(TypeToken::Scalar("usize".into()))),
        owner,
        TypeToken::Result {
            ok: Box::new(TypeToken::from_tree(&byte_slice())),
            err: Box::new(original.clone()),
        },
    ] {
        let mut altered = original.clone();
        let TypeToken::MixedEnum { variants, .. } = &mut altered else {
            unreachable!()
        };
        let VariantShape::Named { fields } = &mut variants[0].1 else {
            unreachable!()
        };
        fields[0].1 = field;
        assert!(error(&altered).is_none());
        assert!(!result(&TypeToken::from_tree(&byte_slice()), &altered));
    }
}

#[test]
fn historical_profiles_reject_direct_and_hidden_new_result_shapes() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    assert!(DataV3.supports(CAPABILITY));
    assert!(!crate::extraction_profile::policy_supports(
        &crate::ProfileId {
            id: DataV3.policy().id,
            version: "3.80.0".into()
        },
        CAPABILITY,
    ));
    let error = original_error();
    for ok in successes() {
        let token = TypeToken::from_tree(&TypeTree::Result {
            ok: Box::new(ok),
            err: Box::new(error.clone()),
        });
        let TypeTree::MixedEnum { nominal, .. } = &error else {
            unreachable!()
        };
        let hidden = TypeToken::Shared(Box::new(TypeToken::Struct {
            nominal: nominal.clone(),
            fields: vec![("hidden".into(), token.clone())],
        }));
        for value in [token, hidden] {
            assert!(uses_result(&value));
            assert!(crate::data_types::parse_for(&value.text().unwrap(), DataV2).is_err());
        }
    }
}
