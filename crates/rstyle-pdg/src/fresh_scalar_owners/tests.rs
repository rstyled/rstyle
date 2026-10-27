use super::*;

fn scalar() -> TypeToken {
    TypeToken::Scalar("u8".into())
}

fn owner() -> TypeToken {
    TypeToken::Struct {
        nominal: crate::concrete_types::Nominal {
            source_identity: Some("shape-only".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: "shape-only-manifest".into(),
            definition_path: "sample::Bytes".into(),
            declaration_digest: "shape-only-declaration".into(),
        },
        fields: vec![(
            "bytes".into(),
            TypeToken::Array {
                element: Box::new(scalar()),
                length: 8,
            },
        )],
    }
}

#[test]
fn scalar_array_shape_does_not_grant_references_nested_owners_or_cleanup() {
    assert!(shape::owner(&owner()));
    for field in [
        owner(),
        TypeToken::Shared(Box::new(scalar())),
        TypeToken::Option(Box::new(scalar())),
        TypeToken::Scalar("unit".into()),
        TypeToken::Array {
            element: Box::new(owner()),
            length: 1,
        },
    ] {
        let TypeToken::Struct { nominal, .. } = owner() else {
            unreachable!()
        };
        let token = TypeToken::Struct {
            nominal,
            fields: vec![("extra".into(), field)],
        };
        assert!(!shape::owner(&token), "{token:?}");
    }
}

#[test]
fn every_nested_owned_occurrence_remains_unavailable_to_ordinary_value_roles() {
    let owner = owner();
    for token in [
        owner.clone(),
        TypeToken::Shared(Box::new(owner.clone())),
        TypeToken::Option(Box::new(owner.clone())),
        TypeToken::Tuple(vec![scalar(), owner.clone()]),
        TypeToken::Array {
            element: Box::new(owner.clone()),
            length: 0,
        },
        TypeToken::Result {
            ok: Box::new(scalar()),
            err: Box::new(owner.clone()),
        },
        TypeToken::Result {
            ok: Box::new(owner.clone()),
            err: Box::new(scalar()),
        },
        TypeToken::MappedIntConversionResult {
            ok: Box::new(owner.clone()),
        },
        TypeToken::NeverConversionResult {
            ok: Box::new(owner.clone()),
        },
    ] {
        assert!(shape::contains(&token, &owner), "{token:?}");
        assert!(returns::rejects(&token, &owner), "{token:?}");
    }
    assert!(!shape::contains(&scalar(), &owner));
}

#[test]
fn same_nominal_with_tampered_fields_still_counts_as_owner_use() {
    let owner = owner();
    let TypeToken::Struct { nominal, .. } = owner.clone() else {
        unreachable!()
    };
    let tampered = TypeToken::Struct {
        nominal,
        fields: vec![("other".into(), scalar())],
    };
    assert!(shape::contains(&tampered, &owner));
    assert!(returns::rejects(&tampered, &owner));
}
