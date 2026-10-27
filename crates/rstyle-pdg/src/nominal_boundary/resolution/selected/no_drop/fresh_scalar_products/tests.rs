use super::super::product_fields::test_support::{facts, origins, resolution};
use super::*;
use crate::concrete_types::{Field, Nominal};

const OWNER: &str = "#[derive(Clone)] pub(crate) struct Accel { bytes: [u8; ACCEL_CAP] }";

fn fixture() -> TypeTree {
    let source: crate::SourceText = OWNER.into();
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: Some("fixture-source".into()),
            package_name: "fixture".into(),
            package_version: "1.0.0".into(),
            manifest_digest: concrete_types::digest(b"manifest"),
            definition_path: "Accel".into(),
            declaration_digest: source.identity().into(),
        },
        source,
        fields: vec![Field {
            position: 0,
            name: "bytes".into(),
            declaration: "[u8; ACCEL_CAP]".into(),
            ty: TypeTree::Array {
                element: Box::new(TypeTree::Scalar("u8".into())),
                length: 8,
            },
        }],
    }
}

fn check(constant: &str, extra: &str) -> Result<CheckedNoDrop, String> {
    let source = format!("{constant} {OWNER} {extra}");
    let mut tree = fixture();
    let mut resolution = resolution(&source, &mut tree);
    resolution.array_views = true;
    let facts = facts(&tree);
    let origins = origins(&resolution, &facts);
    assert!(
        origins
            .declarations
            .iter()
            .all(|origin| origin.builtin_copy.is_none())
    );
    verify_source(&facts, &tree, &resolution, &origins)
}

#[test]
fn clone_only_owner_gets_source_cleanup_without_copy_authority() {
    let checked = check("const ACCEL_CAP: usize = 8;", "").unwrap();
    assert_eq!(checked.nominal().definition_path, "Accel");
}

#[test]
fn full_source_closure_checks_aliased_hidden_and_generated_destructors() {
    for extra in [
        "impl Drop for Accel { fn drop(&mut self) {} }",
        "type Other = Accel; impl Drop for Other { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Accel { fn drop(&mut self) {} } }",
        "macro_rules! cleanup { ($owner:ident) => { impl Drop for $owner { fn drop(&mut self) {} } }; } cleanup!(Accel);",
        "use external::Cleanup; impl Cleanup for Accel {}",
        "#[custom] fn neighbor() {}",
        "unknown!();",
    ] {
        assert!(
            check("const ACCEL_CAP: usize = 8;", extra).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn array_length_is_resolved_from_original_constant_not_observed_shape() {
    for constant in [
        "const ACCEL_CAP: usize = 7;",
        "const ACCEL_CAP: u32 = 8;",
        "const ACCEL_CAP: usize = 4 + 4;",
        "use elsewhere::ACCEL_CAP;",
        "",
    ] {
        assert!(check(constant, "").is_err(), "{constant}");
    }
}

#[test]
fn fresh_product_shape_does_not_authorize_references_owned_fields_or_nesting() {
    for ty in [
        TypeTree::Scalar("unit".into()),
        TypeTree::Shared(Box::new(TypeTree::Scalar("u8".into()))),
        TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into()))),
        TypeTree::Str,
        TypeTree::Option(Box::new(TypeTree::Scalar("u8".into()))),
        fixture(),
        TypeTree::Array {
            element: Box::new(fixture()),
            length: 1,
        },
        TypeTree::Array {
            element: Box::new(TypeTree::Scalar("u8".into())),
            length: concrete_types::MAX_FIELDS as u64 + 1,
        },
    ] {
        assert!(!scalar_field(&ty), "{ty:?}");
    }
}

#[test]
fn source_requires_named_nongeneric_complete_field_inventory() {
    let tree = fixture();
    assert!(supported(&tree));
    for source in [
        "struct Accel<T> { bytes: [u8; ACCEL_CAP], marker: T }",
        "struct Accel<'a> { bytes: &'a [u8] }",
        "struct Accel<const N: usize> { bytes: [u8; N] }",
        "struct Accel where u8: Copy { bytes: [u8; ACCEL_CAP] }",
        "struct Accel([u8; ACCEL_CAP]);",
        "struct Accel { renamed: [u8; ACCEL_CAP] }",
        "struct Accel { bytes: [u8; ACCEL_CAP], hidden: u8 }",
    ] {
        assert!(
            declaration(&syn::parse_str(source).unwrap(), &tree).is_err(),
            "{source}"
        );
    }
}
