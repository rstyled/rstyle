use super::*;

fn selected(source: &str) -> &str {
    let start = source.rfind("fn ").unwrap();
    let span = selected_span(source, start).unwrap();
    &source[span.start as usize..span.end as usize]
}

#[test]
fn exact_preceding_line_skips_only_legacy_attributes_and_docs() {
    for source in [
        "// rstyle-allow(fn_params): ffi\n/// docs\n#[inline]\nfn f() {}",
        "// rstyle-allow(fn_params): ffi\r\n/// docs\r\n#[inline]\r\nfn f() {}",
        "impl X {\n    // rstyle-allow(fn_params): ffi\n    fn f(&self) {}\n}",
    ] {
        assert_eq!(selected(source), "// rstyle-allow(fn_params): ffi");
    }
    for (source, expected) in [
        ("fn f() {}", ""),
        ("// rstyle-allow(fn_params): ffi\n\nfn f() {}", ""),
        ("// candidate\n// ordinary\nfn f() {}", "// ordinary"),
        ("// candidate\n/** doc */\nfn f() {}", "/** doc */"),
        ("// candidate\n#[attr(\n)]\nfn f() {}", ")]"),
        (
            "// π\n\u{2003}// rstyle-allow(fn_length): café\u{2003}\nfn f() {}",
            "// rstyle-allow(fn_length): café",
        ),
    ] {
        assert_eq!(selected(source), expected, "{source}");
    }
}

#[test]
fn physical_lookback_excludes_the_forty_first_line() {
    let forty = format!("// candidate\n{}fn f() {{}}", "/// doc\n".repeat(39));
    assert_eq!(selected(&forty), "// candidate");
    let forty_one = format!("// candidate\n{}fn f() {{}}", "/// doc\n".repeat(40));
    assert_eq!(selected(&forty_one), "");
    assert!(selected_span("π", 1).is_err());
    assert!(selected_span("", 1).is_err());
}
