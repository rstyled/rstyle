use super::super::inspect_external_source as inspect;

#[test]
fn original_same_module_enum_import_keeps_format_macro_owned() {
    let source = "struct Error(()); enum Kind { A } fn f() { use Kind::*; write!(f, \"x\"); }";
    inspect(source, true).unwrap();
    assert!(inspect(source, false).is_err());
    let nested = "struct Error(()); mod nested { enum Kind { A } fn f() { use Kind::*; write!(f, \"x\"); } }";
    // Inline inspection has no separate owning-module entry. Retain that
    // conservative barrier; the production census below supplies the owner.
    assert!(inspect(nested, true).is_err());
    let source = format!("{nested} fn witness() {{}}");
    let (environment, facts) = super::super::imported_macro_tests::fixture(&source);
    super::super::verify_reference_product(&environment, &facts, &vec!["Error".into()]).unwrap();
}

#[test]
fn complete_block_bindings_and_unknown_globs_preserve_rejection() {
    for shadow in [
        "mod Kind {}",
        "type Kind = ();",
        "enum Kind { B }",
        "use core::fmt as Kind;",
        "use self::other::*;",
    ] {
        let source = format!(
            "struct Error(()); enum Kind {{ A }} mod other {{}} fn f() {{ use Kind::*; write!(f, \"x\"); {shadow} }}"
        );
        assert!(inspect(&source, true).is_err(), "{shadow}");
    }
}

#[test]
fn lexical_and_generated_provenance_remain_required() {
    for source in [
        "struct Error(()); #[cfg(any())] enum Kind { A } fn f() { use Kind::*; write!(f, \"x\"); }",
        "struct Error(()); enum Kind { A } fn f<Kind>() { use Kind::*; write!(f, \"x\"); }",
        "struct Error(()); enum Kind { A } fn f() { mod Kind {} { use Kind::*; write!(f, \"x\"); } }",
        "struct Error(()); enum Kind { A } fn f() { use unknown::*; { use Kind::*; write!(f, \"x\"); } }",
        "struct Error(()); enum Kind { A } macro_rules! generated { () => { fn f() { use Kind::*; write!(f, \"x\"); } }; } generated!();",
        "struct Error(()); enum Kind { A } fn f() { mod nested { fn g() { use Kind::*; write!(f, \"x\"); } } }",
        "struct Error(()); enum Kind { A } fn f() { use Kind::*; use self::{unknown::*, Kind::*}; write!(f, \"x\"); }",
    ] {
        assert!(inspect(source, true).is_err(), "{source}");
    }
}

#[test]
fn enum_import_proof_never_omits_hidden_drop() {
    let source = "struct Error(()); enum Kind { A } fn f() { use Kind::*; write!(f, \"x\"); impl Drop for Error { fn drop(&mut self) {} } }";
    assert!(inspect(source, true).is_err());
}
