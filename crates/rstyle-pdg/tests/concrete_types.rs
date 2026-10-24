use rstyle_pdg::concrete_types::*;

fn structure(source: &str) -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: None,
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
        length: u64::MAX,
    };
    validate(&array).unwrap();
    let token = rstyle_pdg::data_types::token(&array).unwrap();
    rstyle_pdg::data_types::parse(&token).unwrap();
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

#[test]
fn declaration_origins_require_exact_owned_struct_ranges() {
    use rstyle_pdg::{ByteSpan, declaration_origins::*};
    let source = "pub struct Counter { pub value: i32 }";
    let tree = structure(source);
    let TypeTree::Struct { nominal, .. } = &tree else {
        unreachable!()
    };
    let file = format!("\u{feff}{source}\r\nmod nested {{ pub struct Other; }}\r\n");
    let start = file.find(source).unwrap() as u32;
    let origin = Origin {
        builtin_copy: None,
        nominal: nominal.clone(),
        source_file: "src/types.rs".into(),
        file_digest: digest(file.as_bytes()),
        file_bytes: file.into(),
        declaration_span: ByteSpan {
            start,
            end: start + source.len() as u32,
        },
    };
    let interface = InterfaceObservations {
        identity: rstyle_pdg::concrete_types::IDENTITY.into(),
        parameters: vec![Observation::Captured { ty: tree }],
        result: Observation::Captured {
            ty: TypeTree::Scalar("unit".into()),
        },
    };
    let origins = Origins {
        generated: Vec::new(),
        identity: IDENTITY.into(),
        declarations: vec![origin],
    };
    verify(&interface, &origins).unwrap();
    for change in [
        "identity",
        "missing",
        "duplicate",
        "digest",
        "span",
        "nominal",
        "comment",
    ] {
        let mut changed = origins.clone();
        change_declaration_origin(&mut changed, change, source);
        assert!(verify(&interface, &changed).is_err(), "{change}");
    }
}

fn change_declaration_origin(
    origins: &mut rstyle_pdg::declaration_origins::Origins,
    change: &str,
    source: &str,
) {
    use rstyle_pdg::ByteSpan;
    let changed = origins;
    match change {
        "identity" => changed.identity.push('x'),
        "missing" => changed.declarations.clear(),
        "duplicate" => changed.declarations.push(changed.declarations[0].clone()),
        "digest" => changed.declarations[0].file_digest = digest(b"different file"),
        "span" => changed.declarations[0].declaration_span.start += 1,
        "nominal" => changed.declarations[0].nominal.manifest_digest = digest(b"other package"),
        "comment" => {
            let entry = &mut changed.declarations[0];
            entry.file_bytes = format!("/* {source} */").into();
            entry.file_digest = digest(entry.file_bytes.as_bytes());
            entry.declaration_span = ByteSpan {
                start: 3,
                end: 3 + source.len() as u32,
            };
        }
        _ => unreachable!(),
    }
}

struct MissingResolution;

impl source_proof::PathResolution for MissingResolution {
    fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
        Err("No selected-unit path resolution".into())
    }
}

#[test]
fn observational_alias_and_const_lengths_cannot_become_source_proofs() {
    for declaration in ["Alias", "[i32; COUNT]"] {
        let source = format!("pub struct Counter {{ pub value: {declaration} }}");
        let mut tree = structure(&source);
        let TypeTree::Struct { fields, .. } = &mut tree else {
            unreachable!()
        };
        fields[0].declaration = declaration.into();
        if declaration.starts_with('[') {
            fields[0].ty = TypeTree::Array {
                element: Box::new(TypeTree::Scalar("i32".into())),
                length: 3,
            };
        }
        validate(&tree).unwrap();
        assert!(source_proof::verify(&tree, &MissingResolution).is_err());
    }
    source_proof::verify(
        &structure("pub struct Counter { pub value: i32 }"),
        &MissingResolution,
    )
    .unwrap();
}

#[test]
fn strict_source_type_proofs_enforce_resource_and_rust_unit_boundaries() {
    let unit = TypeTree::Scalar("unit".into());
    source_proof::verify_spelling("()", &unit, &MissingResolution).unwrap();
    assert!(source_proof::verify_spelling("unit", &unit, &MissingResolution).is_err());
    assert!(
        source_proof::verify_spelling(&" ".repeat(MAX_SOURCE_BYTES + 1), &unit, &MissingResolution)
            .is_err()
    );
}

#[test]
fn option_observations_and_bounded_scalar_semantic_types() {
    for payload in [
        TypeTree::Scalar("usize".into()),
        TypeTree::Shared(Box::new(TypeTree::Scalar("u8".into()))),
    ] {
        let tree = TypeTree::Option(Box::new(payload));
        validate(&tree).unwrap();
        let encoded = serde_json::to_vec(&tree).unwrap();
        assert_eq!(tree, serde_json::from_slice(&encoded).unwrap());
        let token = rstyle_pdg::data_types::token(&tree).unwrap();
        rstyle_pdg::data_types::parse(&token).unwrap();
    }
    for payload in [
        TypeTree::Scalar("unit".into()),
        TypeTree::Scalar("f32".into()),
        TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into()))),
        TypeTree::Option(Box::new(TypeTree::Scalar("u8".into()))),
    ] {
        assert!(validate(&TypeTree::Option(Box::new(payload))).is_err());
    }
}
