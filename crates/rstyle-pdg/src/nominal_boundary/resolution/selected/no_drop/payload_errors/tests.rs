//! Independent cleanup boundaries, including hidden destruction and static storage.
use super::*;
use crate::concrete_types::{MixedVariant, Nominal};

mod reader_products;

fn nominal(name: &str, source: &str) -> Nominal {
    Nominal {
        source_identity: Some("source-only".into()),
        package_name: "sample".into(),
        package_version: "1".into(),
        manifest_digest: crate::source_identity::checksum(b"manifest"),
        definition_path: name.into(),
        declaration_digest: crate::source_identity::checksum(source.as_bytes()),
    }
}

fn field(name: &str, declaration: &str, ty: TypeTree) -> Field {
    Field {
        position: 0,
        name: name.into(),
        declaration: declaration.into(),
        ty,
    }
}

fn tree() -> TypeTree {
    let source = "enum Kind { Overflow { what: &'static str }, Numeric { attempted: u64 } }";
    TypeTree::MixedEnum {
        nominal: nominal("Kind", source),
        source: source.into(),
        variants: vec![
            MixedVariant {
                name: "Overflow".into(),
                position: 0,
                shape: VariantShape::Named {
                    fields: vec![(
                        "what".into(),
                        field(
                            "what",
                            "&'static str",
                            TypeTree::Shared(Box::new(TypeTree::Str)),
                        ),
                    )],
                },
            },
            MixedVariant {
                name: "Numeric".into(),
                position: 1,
                shape: VariantShape::Named {
                    fields: vec![(
                        "attempted".into(),
                        field("attempted", "u64", TypeTree::Scalar("u64".into())),
                    )],
                },
            },
        ],
    }
}

#[test]
fn complete_named_payloads_do_not_require_a_unit_variant_or_copy() {
    let child = tree();
    concrete_types::validate(&child).unwrap();
    assert!(supported(&child));
    for field in child.nominal_fields() {
        static_string(field).unwrap();
    }
    let source = "struct Outer(Kind);";
    let parent = TypeTree::Struct {
        nominal: nominal("Outer", source),
        source: source.into(),
        fields: vec![field("0", "Kind", child)],
    };
    concrete_types::validate(&parent).unwrap();
    assert!(supported(&parent));
}

#[test]
fn source_inventory_cannot_omit_reorder_or_retype_a_variant() {
    for change in 0..4 {
        let mut changed = tree();
        let TypeTree::MixedEnum { variants, .. } = &mut changed else {
            unreachable!()
        };
        match change {
            0 => {
                variants.pop();
            }
            1 => variants.swap(0, 1),
            2 => variants[1].position = 0,
            _ => variants[0].shape = VariantShape::Unit {},
        }
        assert!(
            concrete_types::validate(&changed).is_err(),
            "change {change}"
        );
    }
}

#[test]
fn string_observation_does_not_erase_static_source_obligations() {
    for source in [
        "&str",
        "&'a str",
        "&'static mut str",
        "Alias",
        "&'static Alias",
    ] {
        let value = field("what", source, TypeTree::Shared(Box::new(TypeTree::Str)));
        assert!(static_string(&value).is_err(), "{source}");
    }
    static_string(&field(
        "what",
        "&'static str",
        TypeTree::Shared(Box::new(TypeTree::Str)),
    ))
    .unwrap();
}

#[test]
fn references_owned_storage_and_unregistered_shapes_remain_unavailable() {
    for child in [
        TypeTree::Shared(Box::new(TypeTree::Scalar("u8".into()))),
        TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into()))),
        TypeTree::Option(Box::new(TypeTree::Scalar("u64".into()))),
        TypeTree::Tuple(vec![TypeTree::Scalar("u64".into())]),
    ] {
        assert!(!payload(&child, 0));
        assert!(!supported(&child));
    }
}

fn source_closure(extra: &str, target: &str) -> Result<(), String> {
    let source = format!(
        "struct Outer(Kind); enum Kind {{ Failure {{ what: &'static str }}, Nested {{ err: Child }} }} struct Child(ScalarError); struct ScalarError {{ attempted: u64 }} {extra}"
    );
    let file = syn::parse_file(&source).unwrap();
    let items: Vec<_> = file.items.iter().collect();
    super::super::closure::inspect_scalar_error(&items, &vec![target.into()])
}

#[test]
fn every_recursive_target_preserves_hidden_drop_and_neighbor_checks() {
    for target in ["Outer", "Kind", "Child", "ScalarError"] {
        source_closure("", target).unwrap();
        source_closure(
            "struct Guard; impl Drop for Guard { fn drop(&mut self) {} }",
            target,
        )
        .unwrap();
        for extra in [
            format!("impl Drop for {target} {{ fn drop(&mut self) {{}} }}"),
            format!("fn hidden() {{ impl Drop for {target} {{ fn drop(&mut self) {{}} }} }}"),
            format!("type Alias = {target}; impl Drop for Alias {{ fn drop(&mut self) {{}} }}"),
            "unknown!();".into(),
            "#[custom] fn neighboring() {}".into(),
        ] {
            assert!(source_closure(&extra, target).is_err(), "{target}: {extra}");
        }
    }
}
