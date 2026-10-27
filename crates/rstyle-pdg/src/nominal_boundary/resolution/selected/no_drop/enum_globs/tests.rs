//! A proved enum wildcard in another block cannot introduce a trait alias.
use super::super::*;

const SOURCE: &str = r#"
enum Variants { Ready }
struct Error;
fn neighboring() { use crate::Variants::*; }
impl From<u8> for Error { fn from(_: u8) -> Self { Error } }
"#;

fn inspect(source: &str, reference_product: bool) -> Result<(), String> {
    let file = syn::parse_file(source).unwrap();
    let items = file.items.iter().collect::<Vec<_>>();
    let target = vec!["Error".into()];
    if reference_product {
        closure::inspect_reference_product(
            &items,
            &target,
            crate::data_library::registries().first(),
        )
    } else {
        closure::inspect_closed_imports(&items, &target)
    }
}

#[test]
fn original_enum_globs_preserve_lexical_prelude_only_in_reference_product_mode() {
    for source in [
        SOURCE.to_owned(),
        SOURCE.replace("enum Variants { Ready }", "enum Variants { Ready, From }"),
        SOURCE.replace("crate::Variants::*", "crate::{Variants::*}"),
        SOURCE.replace("crate::Variants::*", "self::Variants::*"),
    ] {
        inspect(&source, true).unwrap();
        let historical = inspect(&source, false).unwrap_err();
        assert!(historical.contains("possible destructor"), "{historical}");
    }
}

#[test]
fn unrelated_original_wildcards_remain_lexical_in_reference_product_mode() {
    for source in [
        SOURCE.replace("crate::Variants::*", "external::*"),
        SOURCE.replace("crate::Variants::*", "crate::Missing::*"),
        SOURCE.replace(
            "enum Variants { Ready }",
            "mod Variants { pub struct Ready; }",
        ),
        SOURCE.replace("crate::Variants::*", "crate::{Variants::*, Missing::*}"),
    ] {
        inspect(&source, true).unwrap();
        assert!(inspect(&source, false).is_err(), "{source}");
    }
}

#[test]
fn named_from_imports_still_poison_prelude_absence() {
    for source in [
        SOURCE.replace(
            "crate::Variants::*",
            "crate::{Variants::*, ops::Drop as From}",
        ),
        SOURCE.replace("crate::Variants::*", "core::ops::Drop as From"),
    ] {
        assert!(inspect(&source, true).is_err(), "{source}");
    }
}

#[test]
fn unknown_original_wildcards_cannot_authorize_traits_or_hidden_cleanup_in_scope() {
    for body in [
        "use external::*; impl From<u16> for Error { fn from(_: u16) -> Self { Error } }",
        "use external::*; impl Drop for Error { fn drop(&mut self) {} }",
        "use crate::{Variants::*, Missing::*}; impl From<u16> for Error { fn from(_: u16) -> Self { Error } }",
        "use external::*; { impl From<u16> for Error { fn from(_: u16) -> Self { Error } } }",
    ] {
        let source = SOURCE.replace("use crate::Variants::*;", body);
        assert!(inspect(&source, true).is_err(), "{source}");
    }
    let siblings = SOURCE.replace(
        "use crate::Variants::*;",
        "{ use external::*; } { impl From<u16> for Error { fn from(_: u16) -> Self { Error } } }",
    );
    inspect(&siblings, true).unwrap();
    assert!(inspect(&siblings, false).is_err());
}

#[test]
fn generated_and_nested_destructor_aliases_still_close_the_proof() {
    for generated in [
        "use core::ops::Drop as From;",
        "use external::*;",
        "type From = Unknown;",
        "pub struct From;",
    ] {
        let source =
            format!("{SOURCE} macro_rules! shadow {{ () => {{ {generated} }}; }} shadow!();");
        assert!(inspect(&source, true).is_err(), "{source}");
    }
    for extra in [
        "fn hidden() { use core::ops::Drop as Cleanup; impl Cleanup for Error { fn drop(&mut self) {} } }",
        "macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
    ] {
        assert!(
            inspect(&format!("{SOURCE} {extra}"), true).is_err(),
            "{extra}"
        );
    }
}
#[test]
fn relative_trait_identity_requires_an_active_original_declaration() {
    for (source, expected) in [
        ("mod sealed { pub trait Sealed {} }", true),
        ("mod sealed { use core::ops::Drop as Sealed; }", false),
        ("mod sealed { pub struct Sealed; }", false),
        ("#[cfg(absent)] mod sealed { pub trait Sealed {} }", false),
        ("mod other { pub trait Sealed {} }", false),
    ] {
        let file = syn::parse_file(source).unwrap();
        let items = file.items.iter().collect::<Vec<_>>();
        let mut scope = super::Scope::inline(&items, &[], true).unwrap();
        let path = vec!["sealed".into(), "Sealed".into()];
        assert_eq!(scope.direct_relative_trait(&path), expected, "{source}");
        assert!(!scope.direct_relative_trait(&["Sealed".into()]));
        scope.unindexed = true;
        assert!(!scope.direct_relative_trait(&path));
    }
}
