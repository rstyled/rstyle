use rstyle_pdg::concrete_types::*;

fn structure(source: &str) -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            package_name: "sample".into(),
            package_version: "1.0.0".into(),
            manifest_digest: digest(b"independent package manifest"),
            definition_path: "sample::Counter".into(),
            declaration_digest: digest(source.as_bytes()),
        },
        source: source.into(),
        fields: vec![Field {
            position: 0,
            name: "value".into(),
            declaration: "i32".into(),
            ty: TypeTree::Scalar("i32".into()),
        }],
    }
}

#[test]
fn concrete_nominal_field_evidence_is_source_backed() {
    let tree = structure("pub struct Counter { pub value: i32 }");
    validate(&tree).unwrap();
    for change in [
        "digest", "name", "order", "spelling", "resolved", "kind", "generic",
    ] {
        let mut changed = tree.clone();
        let TypeTree::Struct {
            nominal,
            source,
            fields,
        } = &mut changed
        else {
            unreachable!()
        };
        match change {
            "digest" => nominal.declaration_digest = digest(b"other declaration"),
            "name" => fields[0].name = "other".into(),
            "order" => fields[0].position = 1,
            "spelling" => fields[0].declaration = "u32".into(),
            "resolved" => fields[0].ty = TypeTree::Scalar("i64".into()),
            "kind" => *source = "pub enum Counter { Value(i32) }".into(),
            "generic" => *source = "pub struct Counter<T> { pub value: T }".into(),
            _ => unreachable!(),
        }
        if matches!(change, "kind" | "generic") {
            nominal.declaration_digest = digest(source.as_bytes());
        }
        assert!(validate(&changed).is_err(), "{change}");
    }
}

#[test]
fn concrete_type_serialization_preserves_reference_referent_and_scalar_width() {
    let signed = TypeTree::Shared(Box::new(TypeTree::Scalar("i32".into())));
    let unsigned = TypeTree::Shared(Box::new(TypeTree::Scalar("u32".into())));
    let slice = TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
        "u8".into(),
    )))));
    let string = TypeTree::Shared(Box::new(TypeTree::Str));
    for ty in [&signed, &unsigned, &slice, &string] {
        validate(ty).unwrap();
        let bytes = serde_json::to_vec(ty).unwrap();
        assert_eq!(*ty, serde_json::from_slice::<TypeTree>(&bytes).unwrap());
    }
    assert_ne!(
        serde_json::to_vec(&signed).unwrap(),
        serde_json::to_vec(&unsigned).unwrap()
    );
    assert_ne!(
        serde_json::to_vec(&slice).unwrap(),
        serde_json::to_vec(&string).unwrap()
    );
    assert!(
        serde_json::from_str::<TypeTree>(r#"{"kind":"Mutable","evidence":{"kind":"Str"}}"#)
            .is_err()
    );
}

#[test]
fn concrete_type_budget_cannot_validate_a_truncated_tree() {
    let mut tree = TypeTree::Scalar("bool".into());
    for _ in 0..=MAX_DEPTH {
        tree = TypeTree::Shared(Box::new(tree));
    }
    assert!(validate(&tree).unwrap_err().contains("budget"));
    let array = TypeTree::Array {
        element: Box::new(TypeTree::Scalar("u8".into())),
        length: MAX_ARRAY + 1,
    };
    assert!(validate(&array).unwrap_err().contains("budget"));
    let tuple = TypeTree::Tuple(vec![TypeTree::Scalar("bool".into()); MAX_ENTRIES]);
    assert!(validate(&tuple).unwrap_err().contains("budget"));
}

#[test]
fn concrete_field_source_offsets_handle_unicode_and_crlf() {
    let source = "pub struct Counter {\r\n    pub 値: i32,\r\n    pub bytes: [u8; 3],\r\n}";
    assert_eq!(
        field_declarations(source).unwrap(),
        vec![
            ("値".into(), "i32".into()),
            ("bytes".into(), "[u8; 3]".into()),
        ]
    );
}

fn composite_field(declaration: &str, ty: TypeTree) -> TypeTree {
    let source = format!("pub struct Counter {{ pub value: {declaration} }}");
    let mut tree = structure(&source);
    let TypeTree::Struct { fields, .. } = &mut tree else {
        unreachable!()
    };
    fields[0].declaration = declaration.into();
    fields[0].ty = ty;

    tree
}

#[test]
fn composite_field_source_structure_checks_widths_lengths_and_referents() {
    let cases = [
        (
            "(i16, bool)",
            TypeTree::Tuple(vec![
                TypeTree::Scalar("i16".into()),
                TypeTree::Scalar("bool".into()),
            ]),
        ),
        (
            "[u8; 0x03]",
            TypeTree::Array {
                element: Box::new(TypeTree::Scalar("u8".into())),
                length: 3,
            },
        ),
        (
            "&'static [u8]",
            TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
                "u8".into(),
            ))))),
        ),
        ("&'static str", TypeTree::Shared(Box::new(TypeTree::Str))),
        ("()", TypeTree::Scalar("unit".into())),
        ("(i32)", TypeTree::Scalar("i32".into())),
    ];
    for (declaration, resolved) in cases {
        validate(&composite_field(declaration, resolved.clone())).unwrap();
        assert!(
            validate(&composite_field(
                declaration,
                TypeTree::Scalar("u64".into())
            ))
            .is_err()
        );
    }
}

#[test]
fn nested_composite_field_tampering_cannot_hide_behind_source_digest() {
    let original = TypeTree::Shared(Box::new(TypeTree::Tuple(vec![
        TypeTree::Array {
            element: Box::new(TypeTree::Scalar("i16".into())),
            length: 3,
        },
        TypeTree::Shared(Box::new(TypeTree::Str)),
    ])));
    let declaration = "&'static ([i16; 3], &'static str)";
    validate(&composite_field(declaration, original.clone())).unwrap();
    let TypeTree::Shared(tuple) = original else {
        unreachable!()
    };
    let TypeTree::Tuple(elements) = *tuple else {
        unreachable!()
    };
    for change in ["width", "length", "arity", "reference", "string"] {
        let mut changed = elements.clone();
        match change {
            "width" => {
                changed[0] = TypeTree::Array {
                    element: Box::new(TypeTree::Scalar("u16".into())),
                    length: 3,
                }
            }
            "length" => {
                changed[0] = TypeTree::Array {
                    element: Box::new(TypeTree::Scalar("i16".into())),
                    length: 4,
                }
            }
            "arity" => {
                changed.pop();
            }
            "reference" => changed[1] = TypeTree::Str,
            "string" => {
                changed[1] = TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(
                    TypeTree::Scalar("u8".into()),
                ))))
            }
            _ => unreachable!(),
        }
        let changed = TypeTree::Shared(Box::new(TypeTree::Tuple(changed)));
        assert!(
            validate(&composite_field(declaration, changed)).is_err(),
            "{change}"
        );
    }
}

#[test]
fn unsupported_source_reference_shapes_remain_outside_observation_checks() {
    for declaration in [
        "&'static mut i32",
        "*const i32",
        "*mut i32",
        "dyn Send",
        "fn() -> i32",
    ] {
        assert!(
            validate(&composite_field(
                declaration,
                TypeTree::Shared(Box::new(TypeTree::Scalar("i32".into())))
            ))
            .is_err(),
            "{declaration}"
        );
    }
}
