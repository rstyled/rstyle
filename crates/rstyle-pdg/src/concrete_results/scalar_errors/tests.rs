use super::*;
use crate::concrete_types::{Field, Observation, TypeTree};

fn nominal(name: &str) -> Nominal {
    // Shape-only identities cannot install Copy or cleanup receipts.
    Nominal {
        source_identity: Some("shape-only".into()),
        package_name: "shape-only".into(),
        package_version: "1".into(),
        manifest_digest: "xxh3-64:0000000000000000".into(),
        definition_path: name.into(),
        declaration_digest: "xxh3-64:0000000000000000".into(),
    }
}

fn product(name: &str, fields: &[(&str, &str)]) -> TypeToken {
    TypeToken::Struct {
        nominal: nominal(name),
        fields: fields
            .iter()
            .map(|(name, ty)| ((*name).into(), TypeToken::Scalar((*ty).into())))
            .collect(),
    }
}

fn error_tree() -> TypeTree {
    TypeTree::Struct {
        nominal: nominal("SmallIndexError"),
        source: "pub struct SmallIndexError { attempted: u64 }".into(),
        fields: vec![Field {
            position: 0,
            name: "attempted".into(),
            declaration: "u64".into(),
            ty: TypeTree::Scalar("u64".into()),
        }],
    }
}

#[test]
fn selected_product_and_named_error_keep_distinct_existing_grammars() {
    let ok = product("SmallIndex", &[("0", "u32")]);
    let err = TypeToken::from_tree(&error_tree());
    assert!(success(&ok));
    assert!(error(&err).is_some());
    assert!(error(&ok).is_none());
    assert!(GRAMMAR.permits(&ok, &err));
    assert!(!crate::concrete_results::copy_errors::GRAMMAR.permits(&ok, &err));
    assert!(!crate::concrete_results::fresh_products::GRAMMAR.permits(&ok, &err));
    assert!(!crate::borrowed_result_types::GRAMMAR.permits(&ok, &err));
}

#[test]
fn products_require_complete_scalar_fields_with_unambiguous_names() {
    for fields in [
        vec![],
        vec![("", "u64")],
        vec![("attempted", "unit")],
        vec![("attempted", "String")],
        vec![("attempted", "f64")],
        vec![("attempted", "u64"), ("attempted", "u32")],
        vec![("0", "u32"), ("attempted", "u64")],
        vec![("1", "u32")],
        vec![("0", "u32"), ("2", "u64")],
        vec![("not a field", "u64")],
    ] {
        let token = product("Invalid", &fields);
        assert!(!success(&token), "{token:?}");
        assert!(error(&token).is_none(), "{token:?}");
    }
    let too_many: Vec<_> = (0..=crate::concrete_types::MAX_FIELDS)
        .map(|position| (position.to_string(), TypeToken::Scalar("u64".into())))
        .collect();
    assert!(!success(&TypeToken::Struct {
        nominal: nominal("TooMany"),
        fields: too_many,
    }));
}

#[test]
fn scalar_products_do_not_imply_nested_or_borrowed_ownership_support() {
    let scalar = TypeToken::Scalar("u64".into());
    for field in [
        product("Nested", &[("value", "u64")]),
        TypeToken::Shared(Box::new(scalar.clone())),
        TypeToken::Slice(Box::new(scalar.clone())),
        TypeToken::Array {
            element: Box::new(scalar.clone()),
            length: 1,
        },
        TypeToken::Option(Box::new(scalar.clone())),
        TypeToken::Tuple(vec![scalar]),
        TypeToken::Str,
    ] {
        let token = TypeToken::Struct {
            nominal: nominal("OwnedError"),
            fields: vec![("attempted".into(), field)],
        };
        assert!(!success(&token));
        assert!(error(&token).is_none());
    }
    let mut token = product("MissingIdentity", &[("attempted", "u64")]);
    let TypeToken::Struct { nominal, .. } = &mut token else {
        unreachable!()
    };
    nominal.source_identity = None;
    assert!(!success(&token));
    assert!(error(&token).is_none());
}

#[test]
fn captured_error_comparison_preserves_field_value_type_and_nominal_identity() {
    let tree = error_tree();
    let token = TypeToken::from_tree(&tree);
    let direct = Observation::Captured { ty: tree.clone() };
    let wrapped = Observation::Captured {
        ty: TypeTree::Result {
            ok: Box::new(TypeTree::Scalar("unit".into())),
            err: Box::new(tree),
        },
    };
    assert!(boundary::captured_error(&direct, &token).is_err());
    let observation = &wrapped;
    assert!(boundary::captured_error(observation, &token).is_ok());
    for changed in [
        product("SmallIndexError", &[("attempted", "u32")]),
        product("SmallIndexError", &[("different", "u64")]),
        product("SmallIndexError", &[]),
        product("OtherError", &[("attempted", "u64")]),
    ] {
        assert!(boundary::captured_error(observation, &changed).is_err());
    }
    assert!(
        boundary::captured_error(
            &Observation::Unsupported {
                reason: "missing original error observation".into(),
            },
            &token,
        )
        .is_err()
    );
}

#[test]
fn new_error_shape_does_not_activate_historical_policy() {
    use crate::extraction_profile::ExtractionProfile;
    let current = ExtractionProfile::DataV3;
    let historical = crate::ProfileId {
        id: current.policy().id,
        version: "3.94.0".into(),
    };
    assert!(!crate::extraction_profile::policy_supports(
        &historical,
        CAPABILITY
    ));
}

fn result_token() -> TypeToken {
    TypeToken::Result {
        ok: Box::new(product("SmallIndex", &[("0", "u32")])),
        err: Box::new(TypeToken::from_tree(&error_tree())),
    }
}

#[test]
fn direct_and_hidden_scalar_results_require_their_own_parser_and_document_policy() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    let historical = crate::ProfileId {
        id: DataV3.policy().id,
        version: "3.96.0".into(),
    };
    for token in [
        result_token(),
        TypeToken::Shared(Box::new(TypeToken::Tuple(vec![result_token()]))),
        TypeToken::Struct {
            nominal: nominal("Stored"),
            fields: vec![("result".into(), result_token())],
        },
    ] {
        assert!(contains_result(&token));
        let text = token.text().unwrap();
        assert_eq!(crate::data_types::parse(&text).unwrap(), token);
        assert!(crate::data_types::parse_for(&text, DataV2).is_err());
        assert!(
            crate::extraction_profile::require_document(
                &historical,
                CAPABILITY,
                || contains_result(&token),
                "Historical scalar Result is closed",
            )
            .is_err()
        );
    }
    // Ordinary scalar products/errors do not imply the new Result vocabulary.
    assert!(!contains_result(&product("SmallIndex", &[("0", "u32")])));
    assert!(!contains_result(&TypeToken::from_tree(&error_tree())));
}

fn observed_result() -> TypeTree {
    let source: crate::SourceText = "pub struct SmallIndex(u32);".into();
    let mut identity = nominal("SmallIndex");
    identity.declaration_digest = source.identity().into();
    let ok = TypeTree::Struct {
        nominal: identity,
        source,
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: "u32".into(),
            ty: TypeTree::Scalar("u32".into()),
        }],
    };
    let mut err = error_tree();
    if let TypeTree::Struct {
        nominal, source, ..
    } = &mut err
    {
        nominal.declaration_digest = source.identity().into();
    }
    TypeTree::Result {
        ok: Box::new(ok),
        err: Box::new(err),
    }
}

#[test]
fn scalar_result_observations_validate_every_original_field_declaration() {
    let original = observed_result();
    assert!(crate::concrete_types::validate(&original).is_ok());
    assert!(super::super::types::return_error(&original).is_some());
    for (name, ty) in [("attempted", "u32"), ("different", "u64")] {
        let mut changed = original.clone();
        let TypeTree::Result { err, .. } = &mut changed else {
            unreachable!()
        };
        let TypeTree::Struct { fields, .. } = err.as_mut() else {
            unreachable!()
        };
        fields[0].name = name.into();
        fields[0].ty = TypeTree::Scalar(ty.into());
        assert!(crate::concrete_types::validate(&changed).is_err());
    }
    let mut changed = original;
    let TypeTree::Result { ok, .. } = &mut changed else {
        unreachable!()
    };
    let TypeTree::Struct { source, .. } = ok.as_mut() else {
        unreachable!()
    };
    *source = "pub struct SmallIndex(u64);".into();
    assert!(crate::concrete_types::validate(&changed).is_err());
}

#[test]
fn fresh_error_storage_detection_preserves_whole_ownership_and_copy_success() {
    let error = TypeToken::from_tree(&error_tree());
    for owned in [
        error.clone(),
        result_token(),
        TypeToken::Option(Box::new(error.clone())),
        TypeToken::Tuple(vec![TypeToken::Scalar("u8".into()), error.clone()]),
        TypeToken::Struct {
            nominal: nominal("Hidden"),
            fields: vec![("error".into(), error.clone())],
        },
    ] {
        assert!(super::super::types::contains_owned(&owned, &error));
    }
    for copy in [
        TypeToken::Scalar("u32".into()),
        product("SmallIndex", &[("0", "u32")]),
    ] {
        assert!(!super::super::types::contains_owned(&copy, &error));
    }
}
