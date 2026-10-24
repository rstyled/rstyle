use super::*;
use crate::concrete_types::{MixedVariant, Nominal, TypeTree, VariantShape};

fn slice(byte: &str) -> TypeToken {
    TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        byte.into(),
    )))))
}

#[test]
fn eager_conversion_preserves_the_exact_shared_payload_and_error_type() {
    let error = TypeToken::Scalar("usize".into());
    let input = TypeToken::Option(Box::new(slice("u8")));
    let output = TypeToken::Result {
        ok: Box::new(slice("u8")),
        err: Box::new(error.clone()),
    };
    types(&input, &error, &output).unwrap();
    for changed in [
        TypeToken::Result {
            ok: Box::new(slice("u16")),
            err: Box::new(error.clone()),
        },
        TypeToken::Result {
            ok: Box::new(slice("u8")),
            err: Box::new(TypeToken::Scalar("u8".into())),
        },
        TypeToken::Option(Box::new(slice("u8"))),
    ] {
        assert!(types(&input, &error, &changed).is_err());
    }
    assert!(types(&TypeToken::Option(Box::new(slice("u16"))), &error, &output).is_err());
}

fn fieldless_error() -> TypeToken {
    let TypeTree::MixedEnum { nominal, .. } = enum_tree() else {
        unreachable!()
    };
    TypeToken::FieldlessEnum {
        nominal,
        variants: vec![("InvalidEncoding".into(), 0), ("InvalidLength".into(), 1)],
    }
}

#[test]
fn scalar_eager_conversion_keeps_saved_usize_and_actual_fieldless_error() {
    let error = fieldless_error();
    let usize_type = TypeToken::Scalar("usize".into());
    let input = TypeToken::Option(Box::new(usize_type.clone()));
    let output = TypeToken::Result {
        ok: Box::new(usize_type.clone()),
        err: Box::new(error.clone()),
    };
    types(&input, &error, &output).unwrap();
    for changed in [
        TypeToken::Result {
            ok: Box::new(TypeToken::Scalar("u32".into())),
            err: Box::new(error.clone()),
        },
        TypeToken::Result {
            ok: Box::new(usize_type.clone()),
            err: Box::new(TypeToken::Scalar("usize".into())),
        },
        TypeToken::Result {
            ok: Box::new(slice("u8")),
            err: Box::new(error.clone()),
        },
    ] {
        assert!(types(&input, &error, &changed).is_err());
    }
    assert!(
        types(
            &TypeToken::Option(Box::new(TypeToken::Scalar("u32".into()))),
            &error,
            &output
        )
        .is_err()
    );
    let mixed = TypeToken::from_tree(&enum_tree());
    let wrong_error = TypeToken::Result {
        ok: Box::new(usize_type),
        err: Box::new(mixed.clone()),
    };
    assert!(types(&input, &mixed, &wrong_error).is_err());
}

#[test]
fn copy_error_semantics_preserve_discriminant_and_eager_unused_error_authority() {
    let checked = error::Error {
        token: fieldless_error(),
        variant: "InvalidLength".into(),
        position: 1,
        discriminant: Some(1),
    };
    let description = error_semantics(&checked);
    assert_eq!(description["rule"], "checked-copy-fieldless-error");
    assert_eq!(description["discriminant"], 1);
    assert_eq!(description["copy_permission"], true);
    assert_eq!(description["cleanup"], "complete-original-copy-proof");
    let eager = semantics_values(
        "ok-saved-scalar-payload",
        "independently-source-proved-copy-fieldless-error",
    );
    assert_eq!(eager["evaluation"], "receiver-then-error-once-even-some");
    assert_eq!(eager["receiver_operand"], 0);
    assert_eq!(eager["error_operand"], 1);
    assert_eq!(eager["some"], "ok-saved-scalar-payload");
    assert_eq!(
        eager["unused_error"],
        "independently-source-proved-copy-fieldless-error"
    );
    assert_eq!(eager["content_read"], false);
    let legacy = error::Error {
        discriminant: None,
        ..checked
    };
    let description = error_semantics(&legacy);
    assert!(description.get("discriminant").is_none());
    assert_eq!(description["copy_permission"], false);
}

#[test]
fn source_contract_rejects_lazy_reordered_or_substituted_forms() {
    for source in [
        "input.get(..4).ok_or(Error::Missing)",
        "(input.get(..4)).ok_or(Error::Missing)",
    ] {
        source::method_expression(&syn::parse_str(source).unwrap()).unwrap();
    }
    for source in [
        "input.ok_or_else(|| Error::Missing)",
        "input.ok_or::<Error>(Error::Missing)",
        "input.ok_or()",
        "input.ok_or(Error::Missing, Error::Other)",
        "ok_or(input, Error::Missing)",
    ] {
        assert!(
            source::method_expression(&syn::parse_str(source).unwrap()).is_err(),
            "{source}"
        );
    }
    for expression in ["Error::Missing", "crate::errors::Error::Missing"] {
        assert_eq!(
            source::qualified_variant(syn::parse_str(expression).unwrap())
                .unwrap()
                .1,
            "Missing"
        );
    }
    for expression in [
        "saved",
        "Error::Missing()",
        "Error::Detailed { size: 0 }",
        "Error::<u8>::Missing",
        "{ Error::Missing }",
    ] {
        assert!(
            source::qualified_variant(syn::parse_str(expression).unwrap()).is_err(),
            "{expression}"
        );
    }
}

#[test]
fn option_body_source_pin_cannot_be_omitted_or_replaced() {
    let mut registry: crate::data_library::Registry =
        serde_json::from_str(include_str!("../../contracts/read-only-lengths-v2.json")).unwrap();
    authority::pinned_source(&registry).unwrap();
    registry
        .result_propagation_source_digests
        .insert("option.rs".into(), "sha256:forged".into());
    assert!(authority::pinned_source(&registry).is_err());
    registry
        .result_propagation_source_digests
        .remove("option.rs");
    assert!(authority::pinned_source(&registry).is_err());
}

fn enum_tree() -> TypeTree {
    TypeTree::MixedEnum {
        nominal: Nominal {
            source_identity: Some("source".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: crate::concrete_types::digest(b"manifest"),
            definition_path: "Error".into(),
            declaration_digest: crate::concrete_types::digest(b"enum"),
        },
        source: "enum Error { Detailed { size: usize }, Missing }".into(),
        variants: vec![
            MixedVariant {
                position: 0,
                name: "Detailed".into(),
                shape: VariantShape::Named { fields: vec![] },
            },
            MixedVariant {
                position: 1,
                name: "Missing".into(),
                shape: VariantShape::Unit {},
            },
        ],
    }
}

#[test]
fn fresh_variant_selection_cannot_treat_named_payloads_as_unit_errors() {
    let original = enum_tree();
    assert_eq!(error::unit_variant(&original, "Missing").unwrap(), 1);
    assert!(error::unit_variant(&original, "Detailed").is_err());
    assert!(error::unit_variant(&original, "Omitted").is_err());
    let mut changed = enum_tree();
    let TypeTree::MixedEnum { variants, .. } = &mut changed else {
        unreachable!()
    };
    variants[1].position = 0;
    assert!(error::unit_variant(&changed, "Missing").is_err());
    let TypeTree::MixedEnum { variants, .. } = &mut changed else {
        unreachable!()
    };
    variants[1].position = 1;
    variants.push(variants[1].clone());
    assert!(error::unit_variant(&changed, "Missing").is_err());
}
