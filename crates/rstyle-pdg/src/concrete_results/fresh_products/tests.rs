use super::*;
use returns::{Role, check_use};

fn nominal() -> crate::concrete_types::Nominal {
    // Shape-only test identity, never a source/Copy/no-drop witness.
    crate::concrete_types::Nominal {
        source_identity: Some("test-only".into()),
        package_name: "shape".into(),
        package_version: "1".into(),
        manifest_digest: "sha256:unused-shape-only".into(),
        definition_path: "Reader".into(),
        declaration_digest: "sha256:unused-shape-only".into(),
    }
}

fn scalar() -> TypeToken {
    TypeToken::Scalar("usize".into())
}

fn product() -> TypeToken {
    TypeToken::Struct {
        nominal: nominal(),
        fields: vec![
            (
                "remaining".into(),
                TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
                    "u8".into(),
                ))))),
            ),
            ("line_width".into(), TypeToken::Option(Box::new(scalar()))),
        ],
    }
}

fn result() -> TypeToken {
    TypeToken::Result {
        ok: Box::new(product()),
        err: Box::new(TypeToken::FieldlessEnum {
            nominal: nominal(),
            variants: vec![("InvalidLength".into(), 0)],
        }),
    }
}

#[test]
fn bounded_product_does_not_expand_scalar_result_or_helper_grammars() {
    let TypeToken::Result { ok, err } = result() else {
        unreachable!()
    };
    assert!(GRAMMAR.permits(&ok, &err));
    assert!(!copy_errors::GRAMMAR.permits(&ok, &err));
    assert!(!crate::borrowed_result_types::GRAMMAR.permits(&ok, &err));
    assert!(!crate::borrowed_result_types::owned_use(&result()));
}

#[test]
fn product_fields_reject_nested_owned_carriers_and_mutable_or_unknown_storage() {
    for field in [
        result(),
        product(),
        TypeToken::Option(Box::new(product())),
        TypeToken::Option(Box::new(result())),
        TypeToken::Scalar("unit".into()),
        TypeToken::Shared(Box::new(scalar())),
        TypeToken::Str,
    ] {
        let TypeToken::Struct {
            nominal,
            mut fields,
        } = product()
        else {
            unreachable!()
        };
        fields.push(("extra".into(), field));
        assert!(!shape::success(&TypeToken::Struct { nominal, fields }));
    }
    let TypeToken::Struct {
        nominal,
        mut fields,
    } = product()
    else {
        unreachable!()
    };
    fields.push(fields[0].clone());
    assert!(!shape::success(&TypeToken::Struct { nominal, fields }));
}

#[test]
fn visibility_finds_hidden_results_without_granting_storage_or_reference_permission() {
    for hidden in [
        result(),
        TypeToken::Option(Box::new(result())),
        TypeToken::Shared(Box::new(result())),
        TypeToken::Tuple(vec![scalar(), result()]),
    ] {
        assert!(used(&hidden));
        assert!(shape::contains_owned(&hidden, &product()));
        assert!(check_use(Some(&hidden), Role::Ordinary, &result(), &product()).is_err());
    }
    assert!(!used(&product()));
    assert!(!used(&scalar()));
}

#[test]
fn value_roles_require_exact_result_and_direct_product_types() {
    let output = result();
    let owner = product();
    assert!(check_use(Some(&output), Role::Return, &output, &owner).is_ok());
    assert!(check_use(Some(&owner), Role::Product, &output, &owner).is_ok());
    assert!(check_use(Some(&scalar()), Role::Ordinary, &output, &owner).is_ok());
    for role in [Role::Return, Role::Product] {
        assert!(check_use(None, role, &output, &owner).is_err());
        assert!(check_use(Some(&scalar()), role, &output, &owner).is_err());
    }
    assert!(check_use(Some(&owner), Role::Return, &output, &owner).is_err());
    assert!(check_use(Some(&output), Role::Product, &output, &owner).is_err());
    assert!(check_use(Some(&owner), Role::Ordinary, &output, &owner).is_err());
}

#[test]
fn current_product_vocabulary_cannot_retroactively_authorize_historical_profiles() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    let historical = crate::ProfileId {
        id: DataV3.policy().id,
        version: "3.88.0".into(),
    };
    for value in [
        result(),
        TypeToken::Shared(Box::new(TypeToken::Tuple(vec![result()]))),
    ] {
        let text = value.text().unwrap();
        assert!(crate::data_types::parse_for(&text, DataV3).is_ok());
        assert!(crate::data_types::parse_for(&text, DataV2).is_err());
        assert!(
            crate::extraction_profile::require_document(
                &historical,
                CAPABILITY,
                || used(&value),
                "Historical fresh product policy is closed",
            )
            .is_err()
        );
    }
}
