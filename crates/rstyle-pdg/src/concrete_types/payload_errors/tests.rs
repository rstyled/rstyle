use super::*;
use crate::concrete_types::{Field, MixedVariant, VariantShape};

fn nominal(name: &str) -> Nominal {
    Nominal {
        source_identity: Some("shape-only".into()),
        package_name: "shape-only".into(),
        package_version: "1".into(),
        manifest_digest: "xxh3-64:0000000000000000".into(),
        definition_path: name.into(),
        declaration_digest: "xxh3-64:0000000000000000".into(),
    }
}

fn error_tree() -> TypeTree {
    TypeTree::Struct {
        nominal: nominal("Failure"),
        source: "struct Failure(Kind);".into(),
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: "Kind".into(),
            ty: TypeTree::MixedEnum {
                nominal: nominal("Kind"),
                source: "enum Kind { Short { what: &'static str } }".into(),
                variants: vec![MixedVariant {
                    position: 0,
                    name: "Short".into(),
                    shape: VariantShape::Named {
                        fields: vec![(
                            "what".into(),
                            Field {
                                position: 0,
                                name: "what".into(),
                                declaration: "&'static str".into(),
                                ty: TypeTree::Shared(Box::new(TypeTree::Str)),
                            },
                        )],
                    },
                }],
            },
        }],
    }
}

fn scalar(name: &str) -> TypeTree {
    TypeTree::Scalar(name.into())
}

#[test]
fn bounded_success_observations_and_tokens_agree() {
    let error = error_tree();
    let token = TypeToken::from_tree(&error);
    for success in [
        scalar("unit"),
        scalar("usize"),
        TypeTree::Tuple(vec![scalar("u32"), scalar("usize")]),
    ] {
        assert!(observed(&success, &error));
        assert!(GRAMMAR.permits(&TypeToken::from_tree(&success), &token));
    }
}

#[test]
fn adjacent_tuple_width_order_arity_and_reference_shapes_stay_closed() {
    let error = error_tree();
    let successes = [
        scalar("u32"),
        scalar("u64"),
        TypeTree::Tuple(vec![]),
        TypeTree::Tuple(vec![scalar("u32")]),
        TypeTree::Tuple(vec![scalar("usize"), scalar("u32")]),
        TypeTree::Tuple(vec![scalar("u16"), scalar("usize")]),
        TypeTree::Tuple(vec![scalar("u64"), scalar("usize")]),
        TypeTree::Tuple(vec![scalar("u32"), scalar("u64")]),
        TypeTree::Tuple(vec![scalar("u32"), scalar("usize"), scalar("usize")]),
        TypeTree::Tuple(vec![
            TypeTree::Shared(Box::new(scalar("u32"))),
            scalar("usize"),
        ]),
        TypeTree::Option(Box::new(scalar("usize"))),
    ];
    for success in successes {
        assert!(!observed(&success, &error), "{success:?}");
        assert!(!GRAMMAR.permits(
            &TypeToken::from_tree(&success),
            &TypeToken::from_tree(&error)
        ));
    }
}

#[test]
fn missing_error_wrapper_field_or_complete_variant_roster_stays_unsupported() {
    let TypeTree::Struct { fields, .. } = error_tree() else {
        unreachable!()
    };
    let kind = fields[0].ty.clone();
    let errors = [
        kind,
        TypeTree::Struct {
            nominal: nominal("Missing"),
            source: "struct Missing;".into(),
            fields: vec![],
        },
        TypeTree::Struct {
            nominal: nominal("Wrong"),
            source: "struct Wrong { kind: Kind }".into(),
            fields: vec![Field {
                name: "kind".into(),
                ..fields[0].clone()
            }],
        },
        TypeTree::Struct {
            nominal: nominal("Empty"),
            source: "struct Empty(Kind);".into(),
            fields: vec![Field {
                ty: TypeTree::MixedEnum {
                    nominal: nominal("Kind"),
                    source: "enum Kind {}".into(),
                    variants: vec![],
                },
                ..fields[0].clone()
            }],
        },
    ];
    for error in errors {
        assert!(!observed(&scalar("unit"), &error), "{error:?}");
    }
}

#[test]
fn observations_do_not_grant_existing_copy_or_fresh_scalar_ownership() {
    // Deliberately shape-only declarations: no resolution/cleanup receipt exists.
    let error = TypeToken::from_tree(&error_tree());
    for success in [
        TypeToken::Scalar("unit".into()),
        TypeToken::Tuple(vec![
            TypeToken::Scalar("u32".into()),
            TypeToken::Scalar("usize".into()),
        ]),
    ] {
        assert!(GRAMMAR.permits(&success, &error));

        assert!(!crate::concrete_results::copy_errors::GRAMMAR.permits(&success, &error));
        assert!(!crate::concrete_results::scalar_errors::GRAMMAR.permits(&success, &error));
        assert!(!crate::concrete_results::fresh_products::GRAMMAR.permits(&success, &error));
        assert!(!crate::borrowed_result_types::GRAMMAR.permits(&success, &error));
    }
}
