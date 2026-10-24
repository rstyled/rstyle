use super::*;
use crate::concrete_types::{Field, MixedVariant, Nominal};

// The original Input declaration, including all fields and its non-Copy derive.
pub(super) const INPUT: &str = "#[derive(Clone)]\npub struct Input<'h> {\n    haystack: &'h [u8],\n    span: Span,\n    anchored: Anchored,\n    earliest: bool,\n}";

fn nominal(name: &str, source: &str) -> Nominal {
    Nominal {
        source_identity: Some("original-package".into()),
        package_name: "sample".into(),
        package_version: "1".into(),
        manifest_digest: concrete_types::digest(b"manifest"),
        definition_path: format!("sample::{name}"),
        declaration_digest: concrete_types::digest(source.as_bytes()),
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

fn structure(name: &str, source: &str, fields: Vec<Field>) -> TypeTree {
    TypeTree::Struct {
        nominal: nominal(name, source),
        source: source.into(),
        fields,
    }
}

fn scalar(token: &str) -> TypeTree {
    TypeTree::Scalar(token.into())
}

fn anchored() -> TypeTree {
    let small = structure(
        "SmallIndex",
        "#[derive(Clone,Copy,Debug,Default,Eq,Hash,PartialEq,PartialOrd,Ord)] #[repr(transparent)] pub struct SmallIndex(u32);",
        vec![field(0, "0", "u32", scalar("u32"))],
    );
    let pattern = structure(
        "PatternID",
        "#[derive(Clone,Copy,Default,Eq,Hash,PartialEq,PartialOrd,Ord)] #[repr(transparent)] pub struct PatternID(SmallIndex);",
        vec![field(0, "0", "SmallIndex", small)],
    );
    let source = "#[derive(Clone,Copy,Debug,Eq,PartialEq)] pub enum Anchored { No, Yes, Pattern(PatternID) }";
    TypeTree::MixedEnum {
        nominal: nominal("Anchored", source),
        source: source.into(),
        variants: vec![
            MixedVariant {
                position: 0,
                name: "No".into(),
                shape: VariantShape::Unit {},
            },
            MixedVariant {
                position: 1,
                name: "Yes".into(),
                shape: VariantShape::Unit {},
            },
            MixedVariant {
                position: 2,
                name: "Pattern".into(),
                shape: VariantShape::SingleTuple {
                    field: field(0, "0", "PatternID", pattern),
                },
            },
        ],
    }
}

pub(super) fn fixture() -> TypeTree {
    let span = structure(
        "Span",
        "#[derive(Clone,Copy,Eq,Hash,PartialEq)] pub struct Span { pub start: usize, pub end: usize }",
        vec![
            field(0, "start", "usize", scalar("usize")),
            field(1, "end", "usize", scalar("usize")),
        ],
    );
    structure(
        "Input",
        INPUT,
        vec![
            field(
                0,
                "haystack",
                "&'h [u8]",
                TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(scalar("u8"))))),
            ),
            field(1, "span", "Span", span),
            field(2, "anchored", "Anchored", anchored()),
            field(3, "earliest", "bool", scalar("bool")),
        ],
    )
}

#[test]
fn original_input_representation_keeps_the_entire_pattern_dependency_chain() {
    let tree = fixture();
    concrete_types::validate(&tree).unwrap();
    assert!(supported(&tree));
    declaration(&syn::parse_str(INPUT).unwrap(), &tree).unwrap();
    assert_eq!(
        tree.nominal_fields()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["haystack", "span", "anchored", "earliest"]
    );
    let anchored = &tree.nominal_fields().nth(2).unwrap().ty;
    assert_eq!(
        anchored.nominal_fields().count(),
        1,
        "Pattern payload is retained even for No construction"
    );
    assert!(copy_shape(anchored, 0));
}

#[test]
fn explicit_source_lifetime_is_required_independently_of_erased_observations() {
    let tree = fixture();
    for source in [
        INPUT.replace("&'h [u8]", "&'static [u8]"),
        INPUT.replace("&'h [u8]", "&[u8]"),
        INPUT.replace("&'h [u8]", "&'h mut [u8]"),
        INPUT.replace("<'h>", "<'h, 'other>"),
    ] {
        assert!(
            declaration(&syn::parse_str(&source).unwrap(), &tree).is_err(),
            "{source}"
        );
    }
}

#[test]
fn owned_unknown_reference_and_cleanup_payload_shapes_do_not_become_copy() {
    for ty in [
        TypeTree::Str,
        TypeTree::Option(Box::new(scalar("unit"))),
        TypeTree::Shared(Box::new(scalar("u8"))),
        TypeTree::Tuple(vec![scalar("u32")]),
        scalar("unit"),
    ] {
        assert!(!copy_shape(&ty, 0));
    }
    let mut tree = fixture();
    let TypeTree::Struct { fields, .. } = &mut tree else {
        unreachable!()
    };
    fields[2].ty = TypeTree::Shared(Box::new(scalar("u8")));
    assert!(!supported(&tree));
}

#[test]
fn missing_pattern_field_or_changed_nested_scalar_is_not_complete_evidence() {
    let mut tree = fixture();
    let TypeTree::Struct { fields, .. } = &mut tree else {
        unreachable!()
    };
    let TypeTree::MixedEnum { variants, .. } = &mut fields[2].ty else {
        unreachable!()
    };
    variants.pop();
    assert!(concrete_types::validate(&tree).is_err());
    let mut tree = fixture();
    let TypeTree::Struct { fields, .. } = &mut tree else {
        unreachable!()
    };
    let TypeTree::Struct { fields, .. } = &mut fields[1].ty else {
        unreachable!()
    };
    fields[1].ty = scalar("u32");
    assert!(concrete_types::validate(&tree).is_err());
}

fn cleanup_source(extra: &str, target: &str) -> Result<(), String> {
    let source = format!("{INPUT} {extra}");
    let file = syn::parse_file(&source).unwrap();
    let items: Vec<_> = file.items.iter().collect();
    closure::inspect(&items, &[], &vec![target.into()])
}

#[test]
fn source_cleanup_is_independent_of_noncopy_shape_and_cannot_skip_hidden_items() {
    cleanup_source("", "Input").unwrap();
    for extra in [
        "impl Drop for Input<'_> { fn drop(&mut self) {} }",
        "type Alias<'a> = Input<'a>; impl Drop for Alias<'_> { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Input<'_> { fn drop(&mut self) {} } }",
        "macro_rules! cleanup { ($owner:ident) => { impl Drop for $owner<'_> { fn drop(&mut self) {} } }; } cleanup!(Input);",
        "use external::Cleanup; impl Cleanup for Input<'_> {}",
        "#[custom] fn neighbor() {}",
        "unknown!();",
    ] {
        assert!(cleanup_source(extra, "Input").is_err(), "{extra}");
    }
}

#[test]
fn copy_payload_cleanup_is_checked_even_when_only_a_unit_variant_is_constructed() {
    for target in ["Span", "Anchored", "PatternID", "SmallIndex"] {
        let source =
            format!("struct {target}; impl Drop for {target} {{ fn drop(&mut self) {{}} }}");
        assert!(cleanup_source(&source, target).is_err(), "{target}");
    }
    assert!(cleanup_source("#[cfg(any())] unknown!();", "Input").is_ok());
}

#[test]
fn source_copy_spelling_and_drop_flags_are_not_positive_copy_evidence() {
    let tree = fixture();
    let child = &tree.nominal_fields().nth(1).unwrap().ty;
    let (nominal, source) = child.nominal_parts().unwrap();
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let facts: HirFnFacts =
        serde_json::from_str(include_str!("../../function_cleanup_tests/facts.json")).unwrap();
    let mut origins = Origins {
        generated: Vec::new(),
        identity: crate::declaration_origins::IDENTITY.into(),
        declarations: vec![],
    };
    assert!(super::super::super::checked_copy(&facts, &origins, child, &item.attrs).is_err());
    origins
        .declarations
        .push(crate::declaration_origins::Origin {
            nominal: nominal.clone(),
            source_file: "/sample/src/lib.rs".into(),
            file_bytes: source.as_str().into(),
            file_digest: concrete_types::digest(source.as_bytes()),
            declaration_span: crate::ByteSpan {
                start: 0,
                end: source.len() as u32,
            },
            builtin_copy: None,
        });
    assert!(!super::super::super::checked_copy(&facts, &origins, child, &item.attrs).unwrap());
}

#[test]
fn scalar_option_is_a_bounded_copy_shape_without_product_copy_authority() {
    assert!(copy_shape(&TypeTree::Option(Box::new(scalar("usize"))), 0));
    for payload in [
        fixture(),
        TypeTree::Option(Box::new(scalar("usize"))),
        TypeTree::Shared(Box::new(scalar("u8"))),
        scalar("unit"),
    ] {
        assert!(!copy_shape(&TypeTree::Option(Box::new(payload)), 0));
    }
}
