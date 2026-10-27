use super::*;
use std::sync::Arc;

const WITNESS: &str = "fn witness() {}";
const OWNER: &str =
    "mod month { pub enum Month { January, February } } pub use crate::month::Month;";

fn source(owner: &str, body: &str) -> String {
    format!(
        "struct Error(()); {owner} mod parsing {{ mod component {{ use crate::Month; {body} }} }} {WITNESS}"
    )
}

fn verify(source: &str) -> Result<(), String> {
    let (original, mut facts) =
        super::super::imported_macro_tests::fixture(&format!("struct Error(()); {WITNESS}"));
    let mut census = original.census().clone();
    census.files[0].bytes = source.into();
    census.files[0].digest = crate::source_identity::checksum(source.as_bytes());
    let environment = Environment::with_options(Arc::new(census), true, Some(&[]), false)?;
    let start = source.find(WITNESS).unwrap() as u32;
    facts.fn_span = ByteSpan {
        start,
        end: start + WITNESS.len() as u32,
    };
    super::super::verify_reference_product(&environment, &facts, &vec!["Error".into()])
}

#[test]
fn original_month_reexport_chain_proves_only_the_enum_glob() {
    let source = source(
        OWNER,
        "fn parse_month() { use Month::*; cfg!(target_endian = \"little\"); }",
    );
    verify(&source).unwrap();
    assert!(super::super::inspect_external_source(&source, false).is_err());
    let renamed = source
        .replace("use crate::Month;", "use crate::Month as Calendar;")
        .replace("use Month::*;", "use Calendar::*;");
    verify(&renamed).unwrap();
}

#[test]
fn terminal_non_enums_missing_owners_and_import_cycles_do_not_prove_a_glob() {
    for owner in [
        "mod month { pub struct Month; } pub use crate::month::Month;",
        "mod month { pub type Month = u8; } pub use crate::month::Month;",
        "pub use external::Month;",
        "pub use crate::Other as Month; pub use crate::Month as Other;",
        "mod other { pub enum Month { January } } pub use crate::missing::Month;",
        "mod month { #[cfg(any())] pub enum Month { January } } pub use crate::month::Month;",
    ] {
        let source = source(
            owner,
            "fn parse_month() { use Month::*; cfg!(target_endian = \"little\"); }",
        );
        assert!(verify(&source).is_err(), "{source}");
    }
}

#[test]
fn independently_closed_empty_module_glob_keeps_its_existing_source_proof() {
    // This is not an enum receipt: the older complete module-source proof
    // independently establishes that the module exports no unknown bindings.
    let source = source("mod Month {}", "fn f() { use Month::*; cfg!(unix); }");
    verify(&source).unwrap();
    let hidden = source.replace("mod Month {}", "mod Month { pub use external::*; }");
    assert!(verify(&hidden).is_err());
}

#[test]
fn imported_enum_proof_preserves_all_lexical_shadow_and_unknown_glob_barriers() {
    for body in [
        "fn f() { use Month::*; cfg!(unix); mod Month {} }",
        "fn f() { use Month::*; cfg!(unix); type Month = (); }",
        "fn f() { use Month::*; cfg!(unix); enum Month { Other } }",
        "fn f() { use Month::*; cfg!(unix); use crate::other as Month; }",
        "fn f() { mod Month {} { use Month::*; cfg!(unix); } }",
        "fn f<Month>() { use Month::*; cfg!(unix); }",
        "fn f() { use Month::*; use unknown::*; cfg!(unix); }",
        "fn f() { use unknown::*; { use Month::*; cfg!(unix); } }",
        "fn f() { use self::{Month::*, unknown::*}; cfg!(unix); }",
        "macro_rules! generated { () => { fn f() { use Month::*; cfg!(unix); } }; } generated!();",
        "fn f() { mod nested { use crate::Month; fn g() { use Month::*; cfg!(unix); } } }",
    ] {
        let source = source(OWNER, body);
        assert!(verify(&source).is_err(), "{source}");
    }
}

#[test]
fn imported_enum_receipt_never_omits_neighboring_nested_or_generated_drop() {
    let owner = "mod month { pub enum Month { January } impl Drop for crate::Error { fn drop(&mut self) {} } } pub use crate::month::Month;";
    assert!(
        verify(&source(
            owner,
            "fn parse_month() { use Month::*; cfg!(unix); }"
        ))
        .is_err()
    );
    for cleanup in [
        "impl Drop for Error { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Error { fn drop(&mut self) {} } }",
        "mod cleanup { impl Drop for crate::Error { fn drop(&mut self) {} } }",
        "macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
    ] {
        let source = format!(
            "{} {cleanup}",
            source(
                OWNER,
                "fn parse_month() { use Month::*; cfg!(target_endian = \"little\"); }"
            )
        );
        assert!(verify(&source).is_err(), "{source}");
    }
}
