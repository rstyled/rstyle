use super::super::product_fields::test_support::{facts, origins, resolution, sources};
use super::*;
use crate::concrete_types::{Field, MixedVariant, Nominal, VariantShape};
use std::collections::BTreeMap;

fn identity(name: &str) -> Nominal {
    Nominal {
        source_identity: None,
        package_name: "shape-only".into(),
        package_version: "0".into(),
        manifest_digest: "unverified".into(),
        definition_path: format!("sample::{name}"),
        declaration_digest: "unverified".into(),
    }
}

fn field(position: usize, name: &str, declaration: &str, ty: TypeTree) -> Field {
    Field {
        position,
        name: name.into(),
        declaration: declaration.into(),
        ty,
    }
}

fn product(lifetime: &str) -> TypeTree {
    TypeTree::Struct {
        nominal: identity("RangeError"),
        source: format!("#[derive(Clone, Copy)] struct RangeError {{ name: &{lifetime} str, conditional: bool }}").into(),
        fields: vec![
            field(0, "name", "&'static str", TypeTree::Shared(Box::new(TypeTree::Str))),
            field(1, "conditional", "bool", TypeTree::Scalar("bool".into())),
        ],
    }
}

fn mixed() -> TypeTree {
    TypeTree::MixedEnum {
        nominal: identity("ParseError"),
        source: "#[derive(Clone, Copy)] enum ParseError { Missing, Range(RangeError) }".into(),
        variants: vec![
            MixedVariant {
                position: 0,
                name: "Missing".into(),
                shape: VariantShape::Unit {},
            },
            MixedVariant {
                position: 1,
                name: "Range".into(),
                shape: VariantShape::SingleTuple {
                    field: field(0, "0", "RangeError", product("'static")),
                },
            },
        ],
    }
}

fn check(mut tree: TypeTree, extra: &str, alter: impl FnOnce(&mut Origins)) -> Result<(), String> {
    let mut declarations = BTreeMap::new();
    sources(&tree, &mut declarations);
    let source = format!(
        "{}\n{extra}",
        declarations.into_values().collect::<Vec<_>>().join("\n")
    );
    let resolution = resolution(&source, &mut tree);
    let facts = facts(&tree);
    let mut origins = origins(&resolution, &facts);
    alter(&mut origins);
    let diagnostic = verify_interface(&facts, &resolution, &origins);
    if matches!(tree, TypeTree::Struct { .. }) && required(&tree) {
        let active = verify_product(&facts, &resolution, &origins, &tree);
        assert_eq!(diagnostic.is_ok(), active.is_ok());
        if let Ok(Some(receipt)) = active {
            assert!(receipt.permits(&tree));
            assert_eq!(
                Some(receipt.cleanup().nominal()),
                tree.nominal_parts().map(|(nominal, _)| nominal)
            );
            assert!(!receipt.permits(&mixed()));
        }
    }
    diagnostic
}

#[test]
fn supplied_static_reference_components_have_complete_recursive_copy_proof() {
    assert!(check(product("'static"), "", |_| {}).is_ok());
    assert!(check(mixed(), "", |_| {}).is_ok());
    let unit = TypeTree::Struct {
        nominal: identity("OtherVariant"),
        source: "#[derive(Clone, Copy)] struct OtherVariant;".into(),
        fields: vec![],
    };
    assert!(check(unit, "", |_| {}).is_ok());
}

#[test]
fn erased_shared_type_does_not_prove_static_source_lifetime() {
    for lifetime in ["", "'a", "'_"] {
        assert!(check(product(lifetime), "", |_| {}).is_err(), "{lifetime}");
    }
    for spelling in ["&str", "&'a str", "&'static mut str", "&'static [u8]"] {
        assert!(source::interface(&TypeTree::Shared(Box::new(TypeTree::Str)), spelling).is_err());
    }
}

#[test]
fn parameter_source_cannot_erase_or_hide_the_supplied_string_lifetime() {
    let string = TypeTree::Shared(Box::new(TypeTree::Str));
    for (parameter, tree, expected) in [
        ("&'static str", string.clone(), true),
        ("&str", string.clone(), false),
        ("&'a str", string.clone(), false),
        ("Alias", string.clone(), false),
        ("(&str,)", TypeTree::Tuple(vec![string.clone()]), false),
        (
            "Option<&str>",
            TypeTree::Option(Box::new(string.clone())),
            false,
        ),
        ("&&str", TypeTree::Shared(Box::new(string)), false),
    ] {
        let function = syn::parse_str(&format!("fn f(value: {parameter}) {{}}")).unwrap();
        let interface = InterfaceObservations {
            identity: concrete_types::IDENTITY.into(),
            parameters: vec![Observation::Captured { ty: tree }],
            result: Observation::Captured {
                ty: TypeTree::Scalar("unit".into()),
            },
        };
        assert_eq!(
            source::parameters(&function, &interface).is_ok(),
            expected,
            "{parameter}"
        );
    }
}

#[test]
fn every_nested_copy_derive_needs_positive_unforged_authority() {
    assert!(
        check(product("'static"), "", |origins| {
            origins.declarations[0].builtin_copy = None;
        })
        .is_err()
    );
    for name in ["ParseError", "RangeError"] {
        assert!(
            check(mixed(), "", |origins| {
                origins
                    .declarations
                    .iter_mut()
                    .find(|origin| origin.nominal.definition_path == name)
                    .unwrap()
                    .builtin_copy = None;
            })
            .is_err(),
            "{name}"
        );
        assert!(
            check(mixed(), "", |origins| {
                origins
                    .declarations
                    .iter_mut()
                    .find(|origin| origin.nominal.definition_path == name)
                    .unwrap()
                    .builtin_copy
                    .as_mut()
                    .unwrap()
                    .derive_definition
                    .crate_hash = "forged".into();
            })
            .is_err(),
            "{name}"
        );
    }
}

#[test]
fn source_closure_retains_hidden_and_generated_destructor_rejections() {
    for extra in [
        "impl Drop for RangeError { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for ParseError { fn drop(&mut self) {} } }",
        "use core::ops::Drop as Cleanup; impl Cleanup for RangeError { fn drop(&mut self) {} }",
        "macro_rules! hidden { ($owner:ident) => { impl Drop for $owner { fn drop(&mut self) {} } }; } hidden!(RangeError);",
        "unknown_sibling!(RangeError);",
    ] {
        assert!(check(mixed(), extra, |_| {}).is_err(), "{extra}");
    }
}

#[test]
fn source_partition_rejects_omission_reordering_and_discriminant_changes() {
    for source in [
        "enum ParseError { Range(RangeError) }",
        "enum ParseError { Range(RangeError), Missing }",
        "enum ParseError { Missing = 1, Range(RangeError) }",
        "enum ParseError { Missing, Range { error: RangeError } }",
    ] {
        let item = syn::parse_str::<syn::Item>(source).unwrap();
        assert!(source::verify(&item, &mixed()).is_err(), "{source}");
    }
}

#[test]
fn shape_bounds_and_noncopy_storage_remain_closed() {
    for unsupported in [
        TypeTree::Scalar("unit".into()),
        TypeTree::Shared(Box::new(TypeTree::Scalar("usize".into()))),
        TypeTree::Shared(Box::new(TypeTree::Shared(Box::new(TypeTree::Str)))),
        TypeTree::Option(Box::new(TypeTree::Scalar("usize".into()))),
        TypeTree::Str,
    ] {
        assert!(!shape::supported(&unsupported));
    }
    let mut tree = product("'static");
    for _ in 0..=concrete_types::MAX_DEPTH {
        tree = TypeTree::Struct {
            nominal: identity("Nested"),
            source: "struct Nested(Inner);".into(),
            fields: vec![field(0, "0", "Inner", tree)],
        };
    }
    assert!(!shape::supported(&tree));
}
