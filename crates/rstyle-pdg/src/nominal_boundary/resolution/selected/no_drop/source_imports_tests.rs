//! The complete cleanup visitor still sees every imported module and item body.
use super::*;

fn reference_product(
    source: &str,
    registry: &'static crate::data_library::Registry,
) -> Result<(), String> {
    let file = syn::parse_file(source).unwrap();
    closure::inspect_reference_product(
        &file.items.iter().collect::<Vec<_>>(),
        &vec!["Error".into()],
        Some(registry),
    )
}

#[test]
fn reference_product_debug_requires_pinned_identity_and_preserves_historical_rejection() {
    let registry = &crate::data_library::registries()[0];
    let source = "struct Error; impl core::fmt::Debug for Error { fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { write!(f, \"error\") } }";
    reference_product(source, registry).unwrap();
    assert!(inspect(source).unwrap_err().contains("possible destructor"));
    for import in ["use core::fmt;", "use core::fmt as format_module;"] {
        let alias = if import.contains(" as ") {
            "format_module"
        } else {
            "fmt"
        };
        let changed = format!(
            "{import} {}",
            source.replace("core::fmt::", &format!("{alias}::"))
        );
        reference_product(&changed, registry).unwrap();
        assert!(inspect(&changed).is_err());
    }
    for changed in [
        format!("{source} mod core {{}}"),
        format!("{source} use external::*;"),
        source.replace("core::fmt::Debug", "external::fmt::Debug"),
        source.replace("core::fmt::Debug", "Debug").replace(
            "struct Error;",
            "struct Error; use core::ops::Drop as Debug;",
        ),
        source.replace(
            "write!(f, \"error\")",
            "{ impl Drop for Error { fn drop(&mut self) {} } write!(f, \"error\") }",
        ),
        format!(
            "{source} macro_rules! hidden {{ () => {{ impl Drop for Error {{ fn drop(&mut self) {{}} }} }}; }} hidden!();"
        ),
    ] {
        assert!(reference_product(&changed, registry).is_err(), "{changed}");
    }
}

#[test]
fn reference_product_debug_cannot_survive_missing_or_changed_source_pins() {
    for root in ["core", "std"] {
        let source = format!("struct Error; impl {root}::fmt::Debug for Error {{}}");
        reference_product(&source, &crate::data_library::registries()[0]).unwrap();
        let paths: &[&str] = if root == "core" {
            &["core/src/fmt/mod.rs"]
        } else {
            &[
                "core/src/fmt/mod.rs",
                "std/src/lib.rs",
                "alloc/src/lib.rs",
                "alloc/src/fmt.rs",
            ]
        };
        for path in paths {
            for replacement in [None, Some("sha256:changed".to_owned())] {
                let mut registry: crate::data_library::Registry = serde_json::from_str(
                    include_str!("../../../../../contracts/read-only-lengths-v2.json"),
                )
                .unwrap();
                registry.cleanup_macro_source_digests.remove(*path);
                if let Some(value) = replacement {
                    registry
                        .cleanup_macro_source_digests
                        .insert((*path).into(), value);
                }
                assert!(
                    reference_product(&source, Box::leak(Box::new(registry))).is_err(),
                    "{root}: {path}"
                );
            }
        }
    }
}

fn inspect(source: &str) -> Result<(), String> {
    let file = syn::parse_file(source).unwrap();
    closure::inspect_closed_imports(
        &file.items.iter().collect::<Vec<_>>(),
        &vec!["Error".into()],
    )
}

const ORIGINAL_STYLE: &str = r#"
use core::{error::Error as ErrorTrait, fmt};
mod compress { pub fn compress() {} }
mod decompress_safe { pub fn decompress() {} }
use self::decompress_safe as decompress;
pub use self::compress::*;
pub use self::decompress::*;
#[derive(Debug)] #[non_exhaustive]
enum Error { TooSmall { expected: usize, actual: usize }, Missing }
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "error") }
}
impl ErrorTrait for Error {}
"#;

#[test]
fn closed_module_exports_and_original_core_aliases_preserve_cleanup_authority() {
    inspect(ORIGINAL_STYLE).unwrap();
    inspect(&ORIGINAL_STYLE.replace(
        "pub fn decompress() {}",
        "pub fn decompress() {} #[test] fn check_token() { assert!(true); }",
    ))
    .unwrap();
    inspect(&ORIGINAL_STYLE.replace("self::compress::*", "compress::*")).unwrap();
    inspect(&ORIGINAL_STYLE.replace("use self::decompress_safe", "use decompress_safe")).unwrap();
}

#[test]
fn unresolved_exports_and_source_authority_spoofs_stay_closed() {
    for source in [
        ORIGINAL_STYLE.replace(
            "pub fn decompress() {}",
            "#[test(unchecked)] fn check_token() {}",
        ),
        ORIGINAL_STYLE.replace(
            "pub fn decompress() {}",
            "#[test] fn check_token() { impl Drop for super::Error { fn drop(&mut self) {} } }",
        ),
        ORIGINAL_STYLE.replace("pub fn compress() {}", "pub use external::Drop as Cleanup;"),
        ORIGINAL_STYLE.replace("pub fn compress() {}", "pub use external::Debug;"),
        ORIGINAL_STYLE.replace("pub fn compress() {}", "pub trait Debug {}"),
        ORIGINAL_STYLE.replace("pub fn compress() {}", "pub mod core {}"),
        ORIGINAL_STYLE.replace("core::{error", "external::{error"),
        format!("{ORIGINAL_STYLE} mod core {{}}"),
        format!(
            "{ORIGINAL_STYLE} type Alias = Error; impl Drop for Alias {{ fn drop(&mut self) {{}} }}"
        ),
        format!(
            "{ORIGINAL_STYLE} fn hidden() {{ impl Drop for Error {{ fn drop(&mut self) {{}} }} }}"
        ),
        format!(
            "{ORIGINAL_STYLE} macro_rules! hidden {{ () => {{ impl Drop for Error {{ fn drop(&mut self) {{}} }} }}; }} hidden!();"
        ),
    ] {
        assert!(inspect(&source).is_err(), "{source}");
    }
}

#[test]
fn module_glob_proof_cannot_be_reused_for_an_unindexed_block_module() {
    let source = format!(
        "{ORIGINAL_STYLE} fn hidden() {{ mod compress {{ pub use external::Debug; }} use self::compress::*; #[derive(Debug)] struct Value; }}"
    );
    assert!(inspect(&source).is_err());
}

#[test]
fn exact_std_debug_reexport_preserves_builtin_derive_authority() {
    let source = "use std::fmt::Debug; #[derive(Debug)] struct Error(());";
    let file = syn::parse_file(source).unwrap();
    assert!(
        closure::inspect(
            &file.items.iter().collect::<Vec<_>>(),
            &[],
            &vec!["Error".into()]
        )
        .is_err()
    );
    inspect(source).unwrap();
    for changed in [
        source.replace("std::fmt", "external::fmt"),
        source.replace("std::fmt", "std::custom"),
        source.replace("Debug;", "Display as Debug;"),
        source.replace("Debug;", "Debug as Clone;"),
        format!("{source} mod std {{}}"),
        format!("{source} mod alloc {{}}"),
        format!("{source} mod core {{}}"),
        format!("{source} use external::*;"),
    ] {
        assert!(inspect(&changed).is_err(), "{changed}");
    }
}

#[test]
fn std_debug_requires_every_reexport_source_pin() {
    let original = &crate::data_library::registries()[0];
    assert!(library_macros::std_debug(Some(original)));
    for path in [
        "std/src/lib.rs",
        "alloc/src/lib.rs",
        "alloc/src/fmt.rs",
        "core/src/fmt/mod.rs",
    ] {
        let mut changed: crate::data_library::Registry = serde_json::from_str(include_str!(
            "../../../../../contracts/read-only-lengths-v2.json"
        ))
        .unwrap();
        changed.cleanup_macro_source_digests.remove(path);
        assert!(!library_macros::std_debug(Some(&changed)), "{path}");
        changed
            .cleanup_macro_source_digests
            .insert(path.into(), "sha256:changed".into());
        assert!(!library_macros::std_debug(Some(&changed)), "{path}");
    }
}

#[test]
fn checked_prelude_from_closes_both_neighbor_conversion_directions() {
    let original = r#"
        mod block { pub enum DecompressError { Missing } }
        mod frame {
            use std::io;
            enum Error { Decode(super::block::DecompressError) }
            impl From<Error> for io::Error { fn from(_: Error) -> Self { todo!() } }
            impl From<io::Error> for Error { fn from(_: io::Error) -> Self { todo!() } }
        }
    "#;
    inspect(original).unwrap();
    for changed in [
        original.replace("use std::io;", "use std::io; use external::Cleanup as From;"),
        original.replace("use std::io;", "use std::io; use external::*;"),
        original.replace("use std::io;", "use std::io; type From = Unknown;"),
        original.replace("use std::io;", "use std::io; mod From {}"),
        original.replace("impl From<Error>", "impl Unknown<Error>"),
        original.replace("fn from(_: Error) -> Self { todo!() }", "fn from(_: Error) -> Self { impl Drop for super::block::DecompressError { fn drop(&mut self) {} } todo!() }"),
    ] {
        assert!(inspect(&changed).is_err(), "{changed}");
    }
}

#[test]
fn generated_trait_bindings_cannot_reuse_implicit_prelude_absence() {
    for generated in [
        "use core::ops::Drop as From;",
        "use external::From;",
        "use external::*;",
        "type From = Unknown;",
        "pub struct From;",
    ] {
        for arguments in ["", "<>", "<u8>"] {
            let source = format!(
                r#"
                struct Error(());
                macro_rules! shadow {{ () => {{ {generated} }}; }}
                shadow!();
                impl From{arguments} for Error {{ fn drop(&mut self) {{}} }}
            "#
            );
            assert!(inspect(&source).is_err(), "{source}");
        }
    }
}
