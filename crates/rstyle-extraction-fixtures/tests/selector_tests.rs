//! Selector-encoding tests: occurrences, overlap, UTF-8, comments, CRLF.
//!
//! These cases are independent of the producer; they pin the token-selector
//! contract (whitespace-insensitive contiguous token runs, 1-based
//! occurrences, byte spans) that every later comparison relies on.

use rstyle_extraction_fixtures::selector::{count_matches, lex, resolve};

fn span_text(source: &str, span: (usize, usize)) -> &str {
    &source[span.0..span.1]
}

#[test]
fn occurrence_numbering_tracks_source_order() {
    let source = "fn f(a: i32) -> i32 { let x = a + 1; x }";

    let first = resolve(source, "a", 1).expect("param a resolves");
    assert_eq!(span_text(source, first), "a");
    assert!(!source[..first.0].contains('{'));

    let second = resolve(source, "a", 2).expect("read a resolves");
    assert_eq!(span_text(source, second), "a");
    assert!(second.0 > first.1);

    assert_eq!(count_matches(source, "a"), 2);
    assert_eq!(resolve(source, "a", 3), None);
    assert_eq!(resolve(source, "a", 0), None);
}

#[test]
fn multi_token_selector_span_includes_whitespace() {
    let source = "fn f(a: i32) -> i32 { let x = a + 1; x }";
    let span = resolve(source, "let x = a + 1;", 1).expect("declaration resolves");

    assert_eq!(span_text(source, span), "let x = a + 1;");
}

#[test]
fn matching_ignores_source_whitespace() {
    let source = "fn f(a: i32) -> i32 { a+1 }";
    let span = resolve(source, "a + 1", 1).expect("spaced selector matches tight source");

    assert_eq!(span_text(source, span), "a+1");
}

#[test]
fn overlapping_selectors_share_bytes_legally() {
    let source =
        "fn f(a: i32, b: bool) -> i32 { let mut x = a; if b { x = 1; } else { x = 2; } x }";
    let read = resolve(source, "b", 2).expect("condition read resolves");
    let predicate = resolve(source, "b", 2).expect("predicate resolves");

    assert_eq!(read, predicate);
    assert_eq!(span_text(source, read), "b");

    let tail = resolve(source, "x", 4).expect("tail read resolves");
    assert_eq!(span_text(source, tail), "x");
    assert_eq!(count_matches(source, "x"), 4);
}

#[test]
fn utf8_offsets_are_bytes_not_chars() {
    let source = "fn f() { let caf\u{e9} = 1; caf\u{e9} }";
    let first = resolve(source, "caf\u{e9}", 1).expect("first binding resolves");
    let second = resolve(source, "caf\u{e9}", 2).expect("second read resolves");

    assert_eq!(span_text(source, first), "caf\u{e9}");
    assert_eq!(span_text(source, second), "caf\u{e9}");
    // `é` is two bytes: the identifier spans six bytes, not five chars.
    assert_eq!(first.1 - first.0, 5);
    assert!(second.0 > first.1);
    assert_eq!(count_matches(source, "caf\u{e9}"), 2);
}

#[test]
fn comments_are_not_identifier_occurrences() {
    let source = "fn f() -> i32 { // x\n let x = 1; x }";

    assert_eq!(count_matches(source, "x"), 2);
    assert_eq!(
        span_text(source, resolve(source, "x", 1).expect("decl")),
        "x"
    );
    assert_eq!(
        span_text(source, resolve(source, "x", 2).expect("tail")),
        "x"
    );
}

#[test]
fn block_comments_are_not_identifier_occurrences() {
    let source = "fn f() -> i32 { /* x /* x */ x */ let x = 1; x }";

    assert_eq!(count_matches(source, "x"), 2);
}

#[test]
fn string_contents_are_not_identifier_occurrences() {
    let source = "fn f() -> i32 { let s = \"a a\"; 1 }";

    assert_eq!(count_matches(source, "a"), 0);
    assert_eq!(count_matches(source, "s"), 1);
}

#[test]
fn crlf_sources_resolve_to_byte_spans() {
    let source = "fn f() -> i32 {\r\n let x = 1;\r\n x\r\n}";

    assert_eq!(count_matches(source, "x"), 2);
    let tail = resolve(source, "x", 2).expect("tail resolves across CRLF");
    assert_eq!(span_text(source, tail), "x");

    let decl = resolve(source, "let x = 1;", 1).expect("decl resolves across CRLF");
    assert_eq!(span_text(source, decl), "let x = 1;");
}

#[test]
fn closing_brace_anchors_implicit_returns() {
    let source = "fn f(a: bool) {}";
    let brace = resolve(source, "}", 1).expect("closing brace resolves");

    assert_eq!(span_text(source, brace), "}");
    assert_eq!(brace, (source.len() - 1, source.len()));
}

#[test]
fn compound_update_selectors_resolve_distinctly() {
    let source = "fn f() -> i32 { let mut x = 1; x += { x = 2; x }; x }";

    assert_eq!(count_matches(source, "x"), 5);
    assert_eq!(count_matches(source, "x = 2"), 1);

    let compound = resolve(source, "x += { x = 2; x }", 1).expect("compound resolves");
    assert_eq!(span_text(source, compound), "x += { x = 2; x }");

    let inner = resolve(source, "x = 2", 1).expect("inner write resolves");
    assert!(inner.0 > compound.0 && inner.1 < compound.1);
}

#[test]
fn integer_suffixes_do_not_split_literals() {
    let source = "fn f() -> i32 { 1i32 }";
    let tokens = lex(source);
    let texts: Vec<&str> = tokens.iter().map(|token| token.text.as_str()).collect();

    assert!(texts.contains(&"1i32"));
    assert!(!texts.contains(&"1"));
}

#[test]
fn empty_selector_never_matches() {
    let source = "fn f() -> i32 { 1 }";

    assert_eq!(resolve(source, "", 1), None);
    assert_eq!(count_matches(source, "   "), 0);
}
