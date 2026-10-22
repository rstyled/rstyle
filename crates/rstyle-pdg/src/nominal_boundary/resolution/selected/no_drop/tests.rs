use super::*;

fn source(text: &str) -> Result<(), String> {
    let file = syn::parse_file(text).unwrap();
    let items: Vec<_> = file.items.iter().collect();
    closure::inspect(&items, &[], &vec!["Error".into()])
}

#[test]
fn complete_unit_owner_does_not_require_copy() {
    source(
        "#[derive(Clone, Debug)] struct Error(()); impl Error { fn new() -> Self { Self(()) } }",
    )
    .unwrap();
}

#[test]
fn direct_nested_and_aliased_destructors_reject() {
    for extra in [
        "use external::Hidden as Cleanup; impl Cleanup for Error { fn drop(&mut self) {} }",
        "impl Drop for Error { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Error { fn drop(&mut self) {} } }",
        "type Alias = Error; impl Drop for Alias { fn drop(&mut self) {} }",
        "use core::ops::Drop as D; impl D for Error { fn drop(&mut self) {} }",
        "use core::ops::Drop as D; use D as D2; impl D2 for Error { fn drop(&mut self) {} }",
        "mod nested { type Alias = super::Error; impl Drop for Alias { fn drop(&mut self) {} } }",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn independently_expanded_simple_macros_cannot_hide_cleanup() {
    source("struct Error(()); macro_rules! identity { ($value:expr) => { $value }; } fn f() { identity!(1); }").unwrap();
    for extra in [
        "macro_rules! hidden { ($ty:ident) => { impl Drop for $ty { fn drop(&mut self) {} } }; } hidden!(Error);",
        "macro_rules! hidden { ($value:expr) => { $value }; } fn f() { hidden!({ impl Drop for Error { fn drop(&mut self) {} } }); }",
        "macro_rules! hidden { ($($tt:tt)*) => { $($tt)* }; } hidden!(impl Drop for Error { fn drop(&mut self) {} });",
        "macro_rules! hidden { () => {}; ($ty:ident) => { impl Drop for $ty { fn drop(&mut self) {} } }; } hidden!(Error);",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn lexical_shadowing_and_unknown_expansion_remain_explicit() {
    for extra in [
        "unknown!();",
        "#[custom] fn f() {}",
        "#[cfg_attr(all(), custom)] fn f() {}",
        "macro_rules! Clone { () => {} }",
        "use external::Clone;",
        "use external::*; fn f() { assert!(true); }",
        "macro_rules! benign { ($x:expr) => { $x }; } fn f() { macro_rules! benign { ($x:expr) => { { impl Drop for Error { fn drop(&mut self) {} } } }; } benign!(1); }",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn inactive_generators_and_unrelated_destructors_remain_distinct() {
    source("struct Error(()); struct Guard; impl Drop for Guard { fn drop(&mut self) {} }")
        .unwrap();
    source("struct Error(()); #[cfg(any())] unknown!();").unwrap();
    source("struct Error(()); macro_rules! erase { ($($tt:tt)*) => { #[cfg(any())] { $($tt)* } }; } fn f() { erase!(impl Drop for Error { fn drop(&mut self) {} }); }").unwrap();
}

#[test]
fn unrelated_names_need_their_lexical_declaration() {
    for extra in [
        "mod elsewhere { struct Guard; } use external::Hidden as Guard; impl Cleanup for Guard {}",
        "struct Guard; fn f() { use external::Hidden as Guard; impl Cleanup for Guard {} }",
        "struct Guard; mod inner { use crate::other::*; impl Cleanup for Guard {} }",
        "struct Guard; type Alias = Error; impl Cleanup for Alias {}",
        "struct Guard; impl Cleanup for (Error) {}",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
    source(
        "struct Error(()); fn f() { struct Guard; impl Drop for Guard { fn drop(&mut self) {} } }",
    )
    .unwrap();
    source("struct Error(()); impl<T> core::ops::Index<usize> for [T] { type Output=T; fn index(&self, i:usize)->&T { &self[i] } }").unwrap();
}

#[test]
fn definition_attributes_and_external_macro_imports_need_authority() {
    for extra in [
        "#[custom] macro_rules! benign { () => {} } benign!();",
        "#[cfg_attr(all(), custom)] macro_rules! benign { () => {} } benign!();",
        "#[cfg_attr(all(), macro_use)] extern crate external; fn f() { assert!(true); }",
        "fn f() { mod hidden; }",
        "macro_rules! hidden { () => { mod external; }; } fn f() { hidden!(); }",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
    source("struct Error(()); #[cfg_attr(any(), macro_use)] extern crate external; #[allow(unused_macros)] macro_rules! benign { () => {} } benign!();").unwrap();
    source("struct Error(()); fn f() { #[cfg(any())] mod hidden; }").unwrap();
}

#[test]
fn standard_roots_and_macro_scopes_do_not_follow_unknown_bindings() {
    for extra in [
        "use external::std::{self}; impl std::error::Error for Error {}",
        "use external::core::{self}; impl core::fmt::Display for Error {}",
        "trait core {}",
        "macro_rules! foo { () => { impl Drop for Error { fn drop(&mut self) {} } }; } mod child { macro_rules! foo { () => {} } } foo!();",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
    let expanded = closure::inherited_expansion("macro_rules! foo { () => { impl Drop for Error { fn drop(&mut self) {} } }; } #[cfg(any())] macro_rules! foo { () => {} } mod child {}", "foo").unwrap();
    assert!(source(&format!("struct Error(()); {expanded}")).is_err());
}

#[test]
fn unused_glob_names_do_not_invent_expansion_effects() {
    source("struct Error(()); fn f() { use external::Kind::*; let x = 0; }").unwrap();
    source(
        "#[derive(Clone)] struct Error(()); fn f() { use external::*; } fn g() { assert!(true); }",
    )
    .unwrap();
    source("struct Error(()); macro_rules! local { () => { 1 }; } fn f() { use external::*; local!(); }").unwrap();
    for extra in [
        "fn f() { use external::*; assert!(true); }",
        "fn f() { use external::*; #[derive(Clone)] struct Local; }",
        "fn f() { use external::*; impl std::error::Error for Error {} }",
        "fn f() { use external::*; impl Cleanup for Error {} }",
        "macro_rules! local { () => { assert!(true) }; } fn f() { use external::*; local!(); }",
        "macro_rules! local { () => { use external::*; assert!(true); }; } fn f() { local!(); }",
        "fn f() { use external::*; { assert!(true); } }",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
}

fn registered_source(text: &str, native: bool) -> Result<(), String> {
    let file = syn::parse_file(text).unwrap();
    let items: Vec<_> = file.items.iter().collect();
    let cfgs = if native {
        vec!["target_thread_local".into()]
    } else {
        vec![]
    };
    closure::inspect_with_library(
        &items,
        &cfgs,
        &vec!["Error".into()],
        crate::data_library::registries().first(),
    )
}

#[test]
fn pinned_scalar_thread_local_inspects_every_original_initializer() {
    registered_source("struct Error(()); use std::thread_local; thread_local! { #[doc=\"id\"] static ID: usize = { let x=1; assert!(x>0); x }; }", true).unwrap();
    for extra in [
        "thread_local! { static ID: usize = { impl Drop for Error { fn drop(&mut self) {} } 0 }; }",
        "thread_local! { #[custom] static ID: usize = 0; }",
        "thread_local! { static ID: Error = Error(()); }",
        "thread_local! { static mut ID: usize = 0; }",
        "type usize = Error; thread_local! { static ID: usize = Error(()); }",
        "macro_rules! thread_local { ($($tt:tt)*) => {}; }",
    ] {
        assert!(
            registered_source(
                &format!("struct Error(()); use std::thread_local; {extra}"),
                true
            )
            .is_err(),
            "{extra}"
        );
    }
    assert!(
        registered_source(
            "struct Error(()); use std::thread_local; thread_local! { static ID: usize = 0; }",
            false
        )
        .is_err()
    );
    assert!(
        registered_source(
            "struct Error(()); thread_local! { static ID: usize = 0; }",
            true
        )
        .is_err()
    );
}

#[test]
fn pinned_named_imports_require_exact_unshadowed_library_roots() {
    registered_source("struct Error(()); use alloc::{vec, format}; fn f() { let _=vec![1]; let _=format!(\"x\"); }", true).unwrap();
    for extra in [
        "use external::vec; fn f() { vec![1]; }",
        "use external::thread_local; thread_local! { static ID:usize=0; }",
        "mod alloc {} use alloc::vec; fn f() { vec![1]; }",
        "mod std {} use std::thread_local; thread_local! { static ID:usize=0; }",
        "use external::std::{self};",
    ] {
        assert!(
            registered_source(&format!("struct Error(()); {extra}"), true).is_err(),
            "{extra}"
        );
    }
    assert!(source("struct Error(()); use alloc::vec; fn f() { vec![1]; }").is_err());
}

#[test]
fn altered_cleanup_source_pins_cannot_enable_macro_authority() {
    let text = include_str!("../../../../../contracts/read-only-lengths-v2.json");
    let mut registry: crate::data_library::Registry = serde_json::from_str(text).unwrap();
    assert!(library_macros::pinned_native(&registry));
    registry
        .cleanup_macro_source_digests
        .insert("std/src/thread/local.rs".into(), "sha256:changed".into());
    assert!(!library_macros::pinned_native(&registry));
    assert!(!library_macros::import(
        &["std".into()],
        "thread_local",
        Some(&registry)
    ));
    registry.target = "unknown-target".into();
    assert!(!library_macros::pinned_native(&registry));
    registry
        .cleanup_macro_source_digests
        .insert("core/src/fmt/mod.rs".into(), "sha256:changed".into());
    assert!(!library_macros::import(
        &["core".into(), "fmt".into()],
        "Debug",
        Some(&registry)
    ));
}

#[test]
fn source_owned_macro_use_survives_coarse_census_poisoning() {
    let expanded = closure::imported_expansion("pub use external::*; #[macro_use] mod macros { macro_rules! debug { () => { impl Drop for Error { fn drop(&mut self) {} } }; } } mod child {}", "debug").unwrap();
    assert!(source(&format!("struct Error(()); {expanded}")).is_err());
    for text in [
        "mod child {} #[macro_use] mod macros { macro_rules! debug { () => {} } }",
        "#[cfg(any())] #[macro_use] mod macros { macro_rules! debug { () => {} } } mod child {}",
        "#[macro_use(debug)] mod macros { macro_rules! debug { () => {} } } mod child {}",
        "#[macro_use] mod macros { unknown!(); } mod child {}",
    ] {
        assert!(
            closure::imported_expansion(text, "debug").is_err(),
            "{text}"
        );
    }
}

#[test]
fn source_owned_enum_globs_do_not_import_macro_or_trait_bindings() {
    for text in [
        "struct Error(()); enum Kind { A } fn f() { use self::Kind::*; assert!(true); }",
        "struct Error(()); enum Kind { A } mod inner { fn f() { use super::Kind::*; assert!(true); } }",
        "struct Error(()); enum Kind { A } mod inner { fn f() { use crate::Kind::*; assert!(true); } }",
        "struct Error(()); mod inner { enum Kind { A } use self::Kind::*; fn f() { assert!(true); } }",
    ] {
        source(text).unwrap();
    }
    for extra in [
        "mod Kind {} fn f() { use self::Kind::*; assert!(true); }",
        "enum Other { A } type Kind=Other; fn f() { use self::Kind::*; assert!(true); }",
        "#[cfg(any())] enum Kind { A } fn f() { use self::Kind::*; assert!(true); }",
        "enum Kind { A } fn f() { use external::Kind::*; assert!(true); }",
        "enum Kind { A } mod inner { mod Kind {} use self::Kind::*; fn f() { assert!(true); } }",
        "enum Kind { A } fn f() { use self::{Kind::*, other::*}; assert!(true); }",
        "#[custom] enum Kind { A } fn f() { use self::Kind::*; assert!(true); }",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
    assert!(source("struct Error(()); enum Kind { A } fn f() { use self::Kind::*; impl Cleanup for Error {} }").is_err());
}

#[test]
fn qualified_alloc_macros_reuse_only_the_exact_pinned_source_contract() {
    registered_source("struct Error(()); fn f() { let _=alloc::vec![1,2]; let _=alloc::format!(\"value {}\", 1); }", true).unwrap();
    for extra in [
        "fn f() { external::vec![1]; }",
        "fn f() { alloc::unknown![1]; }",
        "mod alloc {} fn f() { alloc::vec![1]; }",
        "fn f() { use external::*; alloc::vec![1]; }",
        "fn f() { alloc::vec![{ impl Drop for Error { fn drop(&mut self) {} } 0 }]; }",
        "fn f() { alloc::format!(\"{}\", { impl Drop for Error { fn drop(&mut self) {} } 0 }); }",
    ] {
        assert!(
            registered_source(&format!("struct Error(()); {extra}"), true).is_err(),
            "{extra}"
        );
    }
    assert!(source("struct Error(()); fn f() { alloc::vec![1]; }").is_err());
}

#[test]
fn function_imports_need_an_independently_macro_free_export_name() {
    source("struct Error(()); mod exporter { pub fn value() {} } fn f() { use crate::exporter::value as write; write!(sink, \"x\"); }").unwrap();
    source("struct Error(()); mod exporter { pub fn value() {} #[cfg(any())] pub use external::hidden as value; } fn f() { use crate::exporter::value as write; write!(sink, \"x\"); }").unwrap();
    for body in [
        "pub fn value() {} pub use external::hidden as value;",
        "pub fn value() {} pub use external::{hidden as value};",
        "pub fn value() {} pub use external::*;",
        "pub fn value() {} macro_rules! value { () => {}; }",
        "pub fn value() {} macro_rules! make { () => { pub use external::hidden as value; }; } make!();",
        "pub fn value() {} #[macro_use] mod inner { macro_rules! value { () => {}; } }",
    ] {
        let text = format!(
            "struct Error(()); mod exporter {{ {body} }} fn f() {{ use crate::exporter::value as write; write!(sink, \"x\"); }}"
        );
        assert!(source(&text).is_err(), "{body}");
    }
    assert!(
        source("struct Error(()); fn f() { use external::value as write; write!(sink, \"x\"); }")
            .is_err()
    );
}

#[test]
fn pinned_debug_import_retains_only_exact_builtin_derive_authority() {
    registered_source(
        "struct Error(()); use core::fmt::Debug; #[derive(Debug)] struct Value;",
        true,
    )
    .unwrap();
    for extra in [
        "use external::fmt::Debug;",
        "use core::fmt::Debug as Clone;",
        "mod core {} use core::fmt::Debug;",
        "use core::fmt::Debug; impl Debug for Error {}",
    ] {
        assert!(
            registered_source(&format!("struct Error(()); {extra}"), true).is_err(),
            "{extra}"
        );
    }
    assert!(source("struct Error(()); use core::fmt::Debug;").is_err());
}

#[test]
fn default_marker_is_only_an_inert_unit_variant_attribute() {
    source("struct Error(()); #[derive(Default)] enum Kind { #[default] First, Second }").unwrap();
    for extra in [
        "#[default] struct Other;",
        "enum Kind { #[default(custom)] First }",
        "enum Kind { #[default] Payload(()) }",
        "enum Kind { #[custom] First }",
        "enum Kind { #[default] First = { impl Drop for Error { fn drop(&mut self) {} } 0 } }",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn original_local_traits_exclude_drop_even_for_unresolved_self_aliases() {
    for extra in [
        "trait Local {} type Alias=Error; impl Local for Alias {}",
        "mod api { pub trait Local {} } use crate::api::Local; type Alias=Error; impl Local for Alias {}",
        "trait Local {} mod child { use super::Local; type Alias=super::Error; impl Local for Alias {} }",
        "trait Local {} type Alias=Error; impl crate::Local for Alias {}",
    ] {
        source(&format!("struct Error(()); {extra}")).unwrap();
    }
    for extra in [
        "mod api { pub use external::Drop as Local; } use crate::api::Local; impl Local for Error {}",
        "trait Local {} fn f() { use external::Drop as Local; impl Local for Error {} }",
        "trait Local {} fn f() { use external::*; impl Local for Error {} }",
        "trait Local {} mod child { use external::Local; impl Local for super::Error {} }",
        "trait Local {} unknown!(); impl Local for Error {}",
    ] {
        assert!(
            source(&format!("struct Error(()); {extra}")).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn rooted_index_traits_are_not_cleanup_trait_aliases() {
    for name in ["Index", "IndexMut"] {
        source(&format!(
            "struct Error(()); use external::Owner; impl core::ops::{name}<usize> for Owner {{}}"
        ))
        .unwrap();
        for extra in [
            format!("use external::Owner; impl external::ops::{name}<usize> for Owner {{}}"),
            format!(
                "mod core {{}} use external::Owner; impl core::ops::{name}<usize> for Owner {{}}"
            ),
            format!("use external::*; impl core::ops::{name}<usize> for Error {{}}"),
        ] {
            assert!(
                source(&format!("struct Error(()); {extra}")).is_err(),
                "{extra}"
            );
        }
    }
}

#[test]
fn nested_module_spellings_do_not_borrow_original_module_trait_identity() {
    for extra in [
        "fn f() { mod m { use core::ops::Drop as Local; impl self::Local for crate::Error { fn drop(&mut self) {} } } }",
        "macro_rules! hidden { () => { mod m { use core::ops::Drop as Local; impl self::Local for crate::Error { fn drop(&mut self) {} } } }; } hidden!();",
    ] {
        assert!(
            source(&format!(
                "struct Error(()); mod m {{ trait Local {{}} }} {extra}"
            ))
            .is_err(),
            "{extra}"
        );
    }
}

#[test]
fn standard_range_self_uses_exact_pinned_nominal_binding() {
    registered_source(
        "struct Error(()); use core::ops::Range; impl From<Error> for Range<usize> {}",
        true,
    )
    .unwrap();
    for extra in [
        "use external::Range; impl From<Error> for Range<usize> {}",
        "mod core {} use core::ops::Range; impl From<Error> for Range<usize> {}",
        "use core::ops::Range; fn f() { use external::Range; impl From<Error> for Range<usize> {} }",
    ] {
        assert!(
            registered_source(&format!("struct Error(()); {extra}"), true).is_err(),
            "{extra}"
        );
    }
    assert!(
        source("struct Error(()); use core::ops::Range; impl From<Error> for Range<usize> {}")
            .is_err()
    );
}
