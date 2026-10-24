//! Containment must not supply tag guards or silently discard unknown payloads.
use super::*;
use crate::concrete_types::{Nominal, VariantShape};

fn nominal() -> Nominal {
    Nominal {
        source_identity: Some("selected".into()),
        package_name: "sample".into(),
        package_version: "1".into(),
        manifest_digest: crate::source_identity::checksum(b"manifest"),
        definition_path: "sample::Checked".into(),
        declaration_digest: crate::source_identity::checksum(
            b"enum Checked { Empty, Value(Payload) }",
        ),
    }
}

fn product(reference: bool) -> TypeToken {
    TypeToken::Struct {
        nominal: nominal(),
        fields: vec![
            (
                "name".into(),
                if reference {
                    TypeToken::Shared(Box::new(TypeToken::Str))
                } else {
                    TypeToken::Scalar("usize".into())
                },
            ),
            ("conditional".into(), TypeToken::Scalar("bool".into())),
        ],
    }
}

fn mixed(payload: TypeToken) -> TypeToken {
    TypeToken::MixedEnum {
        nominal: nominal(),
        variants: vec![
            ("Empty".into(), VariantShape::Unit {}),
            ("Value".into(), VariantShape::SingleTuple { field: payload }),
        ],
    }
}

#[test]
fn mixed_payload_containment_does_not_depend_on_operation_admission() {
    let scalar = mixed(product(false));
    assert!(!shared(&scalar));
    assert!(paths(&scalar, Vec::new()).unwrap().is_empty());
    let reference = mixed(product(true));
    assert!(shared(&reference));
    assert!(paths(&reference, Vec::new()).is_err());
    for wrapper in [
        TypeToken::Tuple(vec![reference.clone()]),
        TypeToken::Option(Box::new(reference.clone())),
        mixed(reference),
    ] {
        assert!(shared(&wrapper));
        assert!(paths(&wrapper, Vec::new()).is_err());
    }
}

#[test]
fn both_result_variants_retain_references_without_inventing_guards() {
    for (ok, err) in [
        (product(true), mixed(product(false))),
        (product(false), mixed(product(true))),
        (mixed(product(true)), product(true)),
    ] {
        let result = TypeToken::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        };
        assert!(shared(&result));
        assert!(paths(&result, Vec::new()).is_err());
        let nested = TypeToken::Tuple(vec![result]);
        assert!(shared(&nested));
        assert!(paths(&nested, Vec::new()).is_err());
    }
}

#[test]
fn named_and_tuple_variants_require_the_same_payload_authority() {
    let named = TypeToken::MixedEnum {
        nominal: nominal(),
        variants: vec![
            ("Empty".into(), VariantShape::Unit {}),
            (
                "Value".into(),
                VariantShape::Named {
                    fields: vec![("payload".into(), product(true))],
                },
            ),
        ],
    };
    let tuple = TypeToken::Enum {
        nominal: nominal(),
        variants: vec![("Value".into(), product(true))],
    };
    for value in [named, tuple, TypeToken::Option(Box::new(product(true)))] {
        assert!(shared(&value));
        assert!(paths(&value, Vec::new()).is_err());
    }
}

#[test]
fn later_named_fields_and_variants_cannot_hide_references() {
    let value = TypeToken::MixedEnum {
        nominal: nominal(),
        variants: vec![
            ("Empty".into(), VariantShape::Unit {}),
            (
                "Scalar".into(),
                VariantShape::SingleTuple {
                    field: product(false),
                },
            ),
            (
                "Referenced".into(),
                VariantShape::Named {
                    fields: vec![
                        ("count".into(), TypeToken::Scalar("usize".into())),
                        ("payload".into(), product(true)),
                    ],
                },
            ),
        ],
    };

    assert!(shared(&value));
    assert!(paths(&value, Vec::new()).is_err());
}

#[test]
fn nested_storage_cannot_return_partial_paths_before_a_tagged_reference() {
    let reference = mixed(product(true));
    for value in [
        TypeToken::Array {
            element: Box::new(reference.clone()),
            length: 2,
        },
        TypeToken::Slice(Box::new(reference.clone())),
        TypeToken::Tuple(vec![product(true), reference.clone()]),
        TypeToken::Struct {
            nominal: nominal(),
            fields: vec![("first".into(), product(true)), ("last".into(), reference)],
        },
    ] {
        assert!(shared(&value));
        assert!(paths(&value, vec![Projection::TupleField { position: 3 }]).is_err());
    }
}

fn time_error_shape() -> TypeToken {
    let payload = TypeToken::Struct {
        nominal: nominal(),
        fields: vec![
            ("name".into(), TypeToken::Shared(Box::new(TypeToken::Str))),
            ("is_conditional".into(), TypeToken::Scalar("bool".into())),
        ],
    };
    TypeToken::MixedEnum {
        nominal: nominal(),
        variants: vec![
            ("InsufficientInformation".into(), VariantShape::Unit {}),
            (
                "ComponentRange".into(),
                VariantShape::SingleTuple { field: payload },
            ),
        ],
    }
}

#[test]
fn observable_static_name_shape_never_authorizes_owned_or_borrowed_tagged_values() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    let error = time_error_shape();
    let owner = TypeToken::Struct {
        nominal: nominal(),
        fields: vec![("error".into(), error.clone())],
    };

    for value in [
        error.clone(),
        TypeToken::Tuple(vec![error.clone()]),
        TypeToken::Shared(Box::new(error.clone())),
        TypeToken::Array {
            element: Box::new(error.clone()),
            length: 1,
        },
        TypeToken::Slice(Box::new(error)),
        owner.clone(),
        TypeToken::Shared(Box::new(owner)),
    ] {
        let text = value.text().unwrap();
        assert_eq!(crate::data_types::parse(&text).unwrap(), value);
        assert!(shared(&value));
        for profile in [DataV2, DataV3] {
            assert!(crate::data_types::parse_for(&text, profile).is_err());
        }
    }
}
