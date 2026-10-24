use super::*;
use crate::concrete_types::{Field, Nominal};

const SOURCE: &str =
    "#[derive(Clone, Debug, Eq, PartialEq)] pub struct SmallIndexError { attempted: u64 }";

fn tree() -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: Some("source-shape-only".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: crate::source_identity::checksum(b"manifest"),
            definition_path: "SmallIndexError".into(),
            declaration_digest: crate::source_identity::checksum(SOURCE.as_bytes()),
        },
        source: SOURCE.into(),
        fields: vec![Field {
            position: 0,
            name: "attempted".into(),
            declaration: "u64".into(),
            ty: TypeTree::Scalar("u64".into()),
        }],
    }
}

fn original(source: &str) -> syn::ItemStruct {
    syn::parse_str(source).unwrap()
}

fn closure(extra: &str) -> Result<(), String> {
    let file = syn::parse_file(&format!("{SOURCE} {extra}")).unwrap();
    let items: Vec<_> = file.items.iter().collect();
    super::super::closure::inspect_scalar_error(&items, &vec!["SmallIndexError".into()])
}

#[test]
fn named_scalar_cleanup_accepts_non_copy_error_without_ignoring_neighbors() {
    let observed = tree();
    assert!(supported(&observed));
    declaration(&original(SOURCE), &observed).unwrap();
    closure("").unwrap();
    closure("struct Guard; impl Drop for Guard { fn drop(&mut self) {} }").unwrap();
    closure("#[cfg(any())] unknown!();").unwrap();
}

#[test]
fn named_scalar_cleanup_rejects_missing_changed_aliased_or_generic_source_fields() {
    let observed = tree();
    for changed in [
        SOURCE.replace("attempted: u64", "attempted: u32"),
        SOURCE.replace("attempted: u64", "attempted: Alias"),
        SOURCE.replace("attempted: u64", "attempted: core::primitive::u64"),
        SOURCE.replace("attempted: u64", "attempted: &'static u64"),
        SOURCE.replace("attempted: u64", "attempted: [u64; 1]"),
        SOURCE.replace("attempted: u64", "renamed: u64"),
        SOURCE.replace("attempted: u64", "attempted: u64, hidden: u64"),
        SOURCE.replace("attempted: u64", ""),
        SOURCE.replace("SmallIndexError {", "SmallIndexError<T> {"),
        SOURCE.replace("SmallIndexError {", "SmallIndexError where u64: Copy {"),
    ] {
        assert!(
            declaration(&original(&changed), &observed).is_err(),
            "{changed}"
        );
    }
}

#[test]
fn named_scalar_cleanup_rejects_changed_observation_positions_spelling_or_storage() {
    for fields in [
        vec![],
        vec![Field {
            position: 1,
            ..field()
        }],
        vec![Field {
            name: "forged".into(),
            ..field()
        }],
        vec![Field {
            declaration: "Alias".into(),
            ..field()
        }],
        vec![Field {
            ty: TypeTree::Shared(Box::new(TypeTree::Scalar("u64".into()))),
            ..field()
        }],
    ] {
        let TypeTree::Struct {
            nominal, source, ..
        } = tree()
        else {
            unreachable!()
        };
        let changed = TypeTree::Struct {
            nominal,
            source,
            fields,
        };
        assert!(declaration(&original(SOURCE), &changed).is_err());
    }
}

#[test]
fn named_scalar_source_never_substitutes_field_shape_for_complete_cleanup_closure() {
    for extra in [
        "impl Drop for SmallIndexError { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for SmallIndexError { fn drop(&mut self) {} } }",
        "type Alias = SmallIndexError; impl Drop for Alias { fn drop(&mut self) {} }",
        "use core::ops::Drop as Cleanup; impl Cleanup for SmallIndexError { fn drop(&mut self) {} }",
        "use core::ops::Drop as Cleanup; use Cleanup as Cleanup2; impl Cleanup2 for SmallIndexError { fn drop(&mut self) {} }",
        "mod nested { type Alias = super::SmallIndexError; impl Drop for Alias { fn drop(&mut self) {} } }",
        "macro_rules! hidden { ($ty:ident) => { impl Drop for $ty { fn drop(&mut self) {} } }; } hidden!(SmallIndexError);",
        "unknown!();",
        "macro_rules! Clone { () => {} }",
        "type u64 = Other;",
        "mod u64 {}",
        "use external::Value as u64;",
        "macro_rules! hidden { () => { type u64 = Other; }; } hidden!();",
        "macro_rules! hidden { () => { use external::*; }; } hidden!();",
    ] {
        assert!(closure(extra).is_err(), "{extra}");
    }
}

#[test]
fn cleanup_shape_never_admits_empty_tuple_duplicate_or_nested_fields() {
    for fields in [
        vec![],
        vec![Field {
            name: "0".into(),
            ..field()
        }],
        vec![field(), field()],
        vec![Field {
            ty: TypeTree::Scalar("unit".into()),
            ..field()
        }],
        vec![Field {
            ty: tree(),
            ..field()
        }],
    ] {
        let TypeTree::Struct {
            nominal, source, ..
        } = tree()
        else {
            unreachable!()
        };
        assert!(!supported(&TypeTree::Struct {
            nominal,
            source,
            fields,
        }));
    }
}

fn field() -> Field {
    Field {
        position: 0,
        name: "attempted".into(),
        declaration: "u64".into(),
        ty: TypeTree::Scalar("u64".into()),
    }
}
