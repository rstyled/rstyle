use super::*;

fn success() -> TypeTree {
    TypeTree::Tuple(vec![
        wrapper("ID", wrapper("Inner", scalar("u32"))),
        scalar("usize"),
    ])
}

fn wrapper(name: &str, ty: TypeTree) -> TypeTree {
    let declaration = if name == "Inner" { "u32" } else { "Inner" };
    TypeTree::Struct {
        nominal: nominal(name),
        source: format!("struct {name}({declaration});").into(),
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: declaration.into(),
            ty,
        }],
    }
}

#[test]
fn id_reader_observation_does_not_supply_copy_or_fresh_value_authority() {
    let success = success();
    let token = TypeToken::from_tree(&success);
    let error = TypeToken::from_tree(&error_tree());
    assert!(observed(&success, &error_tree()));
    assert!(id_reader(&token));
    assert!(!reader_product(&token));
    assert!(!crate::concrete_results::copy_errors::GRAMMAR.permits(&token, &error));
    assert!(!crate::concrete_results::scalar_errors::GRAMMAR.permits(&token, &error));
    assert!(!crate::concrete_results::fresh_products::GRAMMAR.permits(&token, &error));
    let value = crate::id_reader_types::product(&success).unwrap();
    let token = TypeToken::from_tree(value);
    assert!(id_value(&token));
    assert!(observed(value, &error_tree()));
    assert!(!crate::concrete_results::copy_errors::GRAMMAR.permits(&token, &error));
    assert!(!crate::concrete_results::fresh_products::GRAMMAR.permits(&token, &error));
}

#[test]
fn id_reader_shape_rejects_width_order_arity_storage_and_wrapper_changes() {
    for change in 0..9 {
        let mut success = success();
        let TypeTree::Tuple(values) = &mut success else {
            unreachable!()
        };
        match change {
            0 => values.reverse(),
            1 => values[1] = scalar("u32"),
            2 => values.push(scalar("usize")),
            3 => values[0] = TypeTree::Shared(Box::new(values[0].clone())),
            _ => {
                let TypeTree::Struct { fields, .. } = &mut values[0] else {
                    unreachable!()
                };
                match change {
                    4 => fields[0].name = "value".into(),
                    5 => fields.push(fields[0].clone()),
                    6 => fields[0].ty = scalar("u32"),
                    _ => {
                        let TypeTree::Struct { fields, .. } = &mut fields[0].ty else {
                            unreachable!()
                        };
                        if change == 7 {
                            fields[0].ty = scalar("u64");
                        } else {
                            fields[0].name = "value".into();
                        }
                    }
                }
            }
        }
        assert!(!observed(&success, &error_tree()), "change {change}");
        assert!(
            !id_reader(&TypeToken::from_tree(&success)),
            "change {change}"
        );
        if change >= 3 {
            let TypeTree::Tuple(values) = &success else {
                unreachable!()
            };
            assert!(
                !id_value(&TypeToken::from_tree(&values[0])),
                "change {change}"
            );
            assert!(!observed(&values[0], &error_tree()), "change {change}");
        }
    }
}

#[test]
fn generated_wrapper_provenance_is_never_erased_into_original_id_authority() {
    for outer in [true, false] {
        let mut success = success();
        let TypeTree::Tuple(values) = &mut success else {
            unreachable!()
        };
        let target = if outer {
            &mut values[0]
        } else {
            let TypeTree::Struct { fields, .. } = &mut values[0] else {
                unreachable!()
            };
            &mut fields[0].ty
        };
        let TypeTree::Struct {
            nominal,
            source,
            fields,
        } = target.clone()
        else {
            unreachable!()
        };
        *target = TypeTree::GeneratedStruct {
            nominal,
            template: source,
            fields,
            definition: Box::new(crate::generated_sources::capture::Definition {
                path: "Generated".into(),
                definition_identity: "generated-id".into(),
                crate_name: "shape-only".into(),
                stable_crate_id: "shape-only".into(),
                local: true,
            }),
        };
        assert!(id_reader(&TypeToken::from_tree(&success)));
        assert!(!observed(&success, &error_tree()));
        let TypeTree::Tuple(values) = &success else {
            unreachable!()
        };
        assert!(id_value(&TypeToken::from_tree(&values[0])));
        assert!(!observed(&values[0], &error_tree()));
    }
}
