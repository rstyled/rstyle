//! Independent source contract: the natural constructors preserve the supplied
//! static string and select a fixed flag; no initializer may execute other work.
use super::*;

#[test]
fn accepts_both_original_constructor_source_shapes() {
    for flag in ["false", "true"] {
        let function = syn::parse_str::<syn::ItemFn>(&format!(
            "pub(crate) const fn new(name: &'static str) -> Self {{ Self {{ name, is_conditional: {flag} }} }}"
        )).unwrap();
        let (parameter, value) = shape::syntax(&function).unwrap();
        assert_eq!(parameter, "name");
        assert_eq!(value.fields.len(), 2);
        assert!(matches!(&value.fields[1].expr, syn::Expr::Lit(literal)
            if matches!(&literal.lit, syn::Lit::Bool(value) if value.value == (flag == "true"))));
    }
}

#[test]
fn rejects_lifetime_signature_and_hidden_body_work() {
    for source in [
        "fn new(name: &str) -> Self { Self { name, flag: false } }",
        "fn new<'a>(name: &'a str) -> Self { Self { name, flag: false } }",
        "fn new(name: &'static mut str) -> Self { Self { name, flag: false } }",
        "unsafe fn new(name: &'static str) -> Self { Self { name, flag: false } }",
        "fn new(name: &'static str) -> Other { Self { name, flag: false } }",
        "fn new(mut name: &'static str) -> Self { Self { name, flag: false } }",
        "fn new(name: &'static str) -> Self { log(); Self { name, flag: false } }",
        "fn new(name: &'static str) -> Self { return Self { name, flag: false }; }",
        "fn new(name: &'static str) -> Self { Self { name, ..default() } }",
        "fn new(name: &'static str) -> Self { make!(name) }",
        "fn new(name: &'static str) -> Self { Self { name: helper(name), flag: false } }",
        "fn new(name: &'static str) -> Self { Self { name, flag: callback() } }",
        "fn new(name: &'static str) -> Self { Self { name, flag: mac!() } }",
        "fn new(name: &'static str) -> Self { Self { name, flag: { false } } }",
        "fn new(name: &'static str) -> Self { Self { name, flag: false, extra: true } }",
        "fn new(name: &'static str) -> Self { Self { name } }",
        "fn new(name: &'static str) -> Self { Self { name: \"invented\", flag: false } }",
        "fn new(name: &'static str) -> Self { Self { #[cfg(any())] name, flag: false } }",
    ] {
        let function = syn::parse_str::<syn::ItemFn>(source).unwrap();
        assert!(shape::syntax(&function).is_err(), "{source}");
    }
}

fn original_product() -> TypeTree {
    use crate::concrete_types::{Field, Nominal};
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: None,
            package_name: "source-shape-only".into(),
            package_version: "0".into(),
            manifest_digest: "unverified".into(),
            definition_path: "ComponentRange".into(),
            declaration_digest: "unverified".into(),
        },
        source: "struct ComponentRange { name: &'static str, is_conditional: bool }".into(),
        fields: vec![
            Field {
                position: 0,
                name: "name".into(),
                declaration: "&'static str".into(),
                ty: TypeTree::Shared(Box::new(TypeTree::Str)),
            },
            Field {
                position: 1,
                name: "is_conditional".into(),
                declaration: "bool".into(),
                ty: TypeTree::Scalar("bool".into()),
            },
        ],
    }
}

#[test]
fn declaration_proof_requires_original_static_lifetime_and_complete_ordered_fields() {
    let tree = original_product();
    let original = syn::parse_str::<syn::ItemStruct>(
        "struct ComponentRange { name: &'static str, is_conditional: bool }",
    )
    .unwrap();
    assert!(shape::declaration(&original, &tree).is_ok());
    for source in [
        "struct ComponentRange<'a> { name: &'a str, is_conditional: bool }",
        "struct ComponentRange { name: &'static mut str, is_conditional: bool }",
        "struct ComponentRange { name: Alias, is_conditional: bool }",
        "struct ComponentRange { is_conditional: bool, name: &'static str }",
        "struct ComponentRange { name: &'static str }",
        "struct ComponentRange { name: &'static str, is_conditional: bool, hidden: Owned }",
        "struct ComponentRange { name: &'static str, renamed: bool }",
        "struct ComponentRange { name: &'static str, is_conditional: u8 }",
    ] {
        let declaration = syn::parse_str::<syn::ItemStruct>(source).unwrap();
        assert!(shape::declaration(&declaration, &tree).is_err(), "{source}");
    }
}
