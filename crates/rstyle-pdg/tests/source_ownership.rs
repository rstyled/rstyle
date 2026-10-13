//! Source-only ownership expectations, independent of producer graphs.

use rstyle_pdg::source_evidence::functions;

#[test]
fn nested_functions_have_distinct_exact_owners() {
    let source = "#[inline]\npub fn outer() { fn inner() {} inner(); }";
    let definitions = functions(source).unwrap();
    assert_eq!(definitions.len(), 2);
    assert_eq!(
        definitions[0].function_source,
        "pub fn outer() { fn inner() {} inner(); }"
    );
    assert_eq!(definitions[1].function_source, "fn inner() {}");
    assert_eq!(definitions[0].function_span.start, 10);
    assert_eq!(definitions[1].function_span.start, 27);
}

#[test]
fn same_named_conditional_and_method_owners_remain_distinct() {
    let source =
        "#[cfg(a)] fn f() {}\n#[cfg(b)] fn f() {}\nstruct S; impl S { pub fn f(&self) {} }";
    let definitions = functions(source).unwrap();
    assert_eq!(definitions.len(), 3);
    assert!(definitions.iter().all(|definition| definition.name == "f"));
    assert_eq!(definitions[0].function_span.start, 10);
    assert_eq!(definitions[1].function_span.start, 30);
    assert_eq!(definitions[2].function_source, "pub fn f(&self) {}");
}

#[test]
fn bom_and_crlf_offsets_are_original_file_bytes() {
    let source = "\u{feff}#[inline]\r\npub fn f() {\r\n    fn g() {}\r\n}\r\n";
    let definitions = functions(source).unwrap();
    assert_eq!(definitions.len(), 2);
    assert_eq!(definitions[0].function_span.start, 14);
    assert_eq!(
        definitions[0].function_source,
        "pub fn f() {\r\n    fn g() {}\r\n}"
    );
    assert_eq!(definitions[1].function_span.start, 32);
    assert_eq!(definitions[1].function_source, "fn g() {}");
}
