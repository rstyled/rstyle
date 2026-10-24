use super::inspect_external_source as inspect;

#[test]
fn exact_original_lint_and_format_metadata_keep_historical_cleanup_closed() {
    for attribute in [
        "expect(deprecated)",
        "expect(missing_docs)",
        "expect(clippy::needless_return, reason = \"reviewed\")",
        "rustfmt::skip",
    ] {
        let source = format!("struct Error(()); #[{attribute}] fn f() {{}}");
        assert!(inspect(&source, true).is_ok(), "{attribute}");
        assert!(inspect(&source, false).is_err(), "{attribute}");
    }
    for attribute in [
        "expect()",
        "expect(missing_docs = true)",
        "expect(deprecated, reason = concat!(\"x\"))",
        "custom::expect(deprecated)",
        "rustfmt::skip()",
        "rustfmt::skip = \"x\"",
        "other::skip",
    ] {
        let source = format!("struct Error(()); #[{attribute}] fn f() {{}}");
        assert!(inspect(&source, true).is_err(), "{attribute}");
    }
}

#[test]
fn lint_metadata_does_not_omit_neighboring_or_generated_drop_implementations() {
    for source in [
        "struct Error(()); #[expect(deprecated)] fn f() {} impl Drop for Error { fn drop(&mut self) {} }",
        "struct Error(()); #[expect(deprecated)] fn f() { impl Drop for Error { fn drop(&mut self) {} } }",
        "struct Error(()); #[rustfmt::skip] macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
    ] {
        assert!(inspect(source, true).is_err(), "{source}");
    }
}

#[test]
fn diagnostic_note_is_literal_metadata_with_no_cleanup_permission() {
    let source = "struct Error(()); #[diagnostic::on_unimplemented(note = \"numeric literals\")] trait Helper {}";
    inspect(source, true).unwrap();
    assert!(inspect(source, false).is_err());
    for attribute in [
        "diagnostic::on_unimplemented(note = concat!(\"x\"))",
        "diagnostic::on_unimplemented(note = 1)",
        "diagnostic::on_unimplemented(label = \"x\")",
        "diagnostic::on_unimplemented(note = \"x\", note = \"y\")",
        "custom::diagnostic::on_unimplemented(note = \"x\")",
    ] {
        let source = format!("struct Error(()); #[{attribute}] trait Helper {{}}");
        assert!(inspect(&source, true).is_err(), "{attribute}");
    }
    let hidden = format!("{source} impl Drop for Error {{ fn drop(&mut self) {{}} }}");
    assert!(inspect(&hidden, true).is_err());
}
