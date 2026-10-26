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

fn reader_success() -> TypeTree {
    TypeTree::Tuple(vec![
        TypeTree::Struct {
            nominal: nominal("Flags"),
            source: "struct Flags { a: bool, b: bool, c: bool }".into(),
            fields: ["a", "b", "c"]
                .into_iter()
                .enumerate()
                .map(|(position, name)| Field {
                    position,
                    name: name.into(),
                    declaration: "bool".into(),
                    ty: scalar("bool"),
                })
                .collect(),
        },
        scalar("usize"),
    ])
}

#[test]
fn bounded_success_observations_and_tokens_agree() {
    let error = error_tree();
    let token = TypeToken::from_tree(&error);
    for success in [
        scalar("unit"),
        scalar("usize"),
        TypeTree::Tuple(vec![scalar("u32"), scalar("usize")]),
        reader_success(),
    ] {
        assert!(observed(&success, &error));
        assert!(GRAMMAR.permits(&TypeToken::from_tree(&success), &token));
    }
}

#[test]
fn reader_product_bounds_do_not_admit_adjacent_products() {
    let error = error_tree();
    for change in 0..9 {
        let mut changed = reader_success();
        let TypeTree::Tuple(values) = &mut changed else {
            unreachable!()
        };
        match change {
            0 => values.reverse(),
            1 => values[1] = scalar("u32"),
            2 => values.push(scalar("usize")),
            3 => values[0] = TypeTree::Shared(Box::new(values[0].clone())),
            4 => values[0] = TypeTree::Tuple(vec![scalar("bool"); 3]),
            _ => {
                let TypeTree::Struct { fields, .. } = &mut values[0] else {
                    unreachable!()
                };
                match change {
                    5 => {
                        fields.pop();
                    }
                    6 => fields.push(fields[0].clone()),
                    7 => fields[1].ty = scalar("u8"),
                    _ => fields[1].ty = TypeTree::Shared(Box::new(scalar("bool"))),
                }
            }
        }
        assert!(!observed(&changed, &error), "change {change}");
        assert!(
            !reader_product(&TypeToken::from_tree(&changed)),
            "change {change}"
        );
    }
}

#[test]
fn generated_product_observation_is_not_an_ordinary_success_struct() {
    let mut success = reader_success();
    let TypeTree::Tuple(values) = &mut success else {
        unreachable!()
    };
    let TypeTree::Struct {
        nominal,
        source,
        fields,
    } = values[0].clone()
    else {
        unreachable!()
    };
    values[0] = TypeTree::GeneratedStruct {
        nominal,
        template: source,
        fields,
        definition: Box::new(crate::generated_sources::capture::Definition {
            path: "Flags".into(),
            definition_identity: "generated-flags".into(),
            crate_name: "shape-only".into(),
            stable_crate_id: "shape-only".into(),
            local: true,
        }),
    };
    // Canonical tokens erase generation; the full observation retains this bound.
    assert!(reader_product(&TypeToken::from_tree(&success)));
    assert!(!observed(&success, &error_tree()));
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
        TypeToken::from_tree(&reader_success()),
    ] {
        assert!(GRAMMAR.permits(&success, &error));

        assert!(!crate::concrete_results::copy_errors::GRAMMAR.permits(&success, &error));
        assert!(!crate::concrete_results::scalar_errors::GRAMMAR.permits(&success, &error));
        assert!(!crate::concrete_results::fresh_products::GRAMMAR.permits(&success, &error));
        assert!(!crate::borrowed_result_types::GRAMMAR.permits(&success, &error));
    }
}
