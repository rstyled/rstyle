use super::*;

fn offsets(source: &str, language: &str) -> Vec<usize> {
    ordinary(scan(source, vec![], language))
}

fn ordinary(comments: Vec<(usize, CommentKind)>) -> Vec<usize> {
    comments
        .into_iter()
        .filter(|(_, kind)| *kind == CommentKind::Ordinary)
        .map(|(offset, _)| offset)
        .collect()
}

#[test]
fn ordinary_line_and_nested_block_comments_are_reported_once() {
    let source = "x(); // why\n/* outer /* nested */ end */ y(); // tail";
    assert_eq!(offsets(source, "rust"), [5, 12, 46]);
}

#[test]
fn documentation_and_tool_directives_stay_allowed() {
    for source in [
        "/// docs\n//! module\n/** docs */\n/*! module */",
        "//go:noinline\n//nolint:unused\n/* NOLINT */\n// rstyle-allow(no_comments): required",
        "// clang-format off\n// rustfmt::skip",
    ] {
        assert!(offsets(source, "rust").is_empty(), "{source}");
    }
    assert!(
        offsets(
            "# noqa\n# type: ignore\n# fmt: off\n# pyright: ignore",
            "python"
        )
        .is_empty()
    );
    assert_eq!(offsets("//// ordinary\n/*** ordinary */", "rust"), [0, 14]);
}

#[test]
fn compiler_literal_spans_protect_strings_and_docstrings() {
    let source = "r###\"// not a comment /* either */\"### // ordinary";
    let end = source.find(" // ordinary").unwrap();
    assert_eq!(
        ordinary(scan(source, std::iter::once(0..end).collect(), "rust")),
        [end + 1]
    );
    let source = "\"\"\"# docstring\n# still a string\"\"\"\n# ordinary";
    let end = source.find("\n# ordinary").unwrap();
    assert_eq!(
        ordinary(scan(source, std::iter::once(0..end).collect(), "python")),
        [end + 1]
    );
}

#[test]
fn comment_syntax_depends_on_the_language() {
    assert_eq!(offsets("a // b # ordinary", "python"), [7]);
    assert!(offsets("#include <x>\n//go:noinline", "go").is_empty());
    assert_eq!(offsets("/* outer /* not nested */ // tail", "go"), [0, 26]);
    assert_eq!(offsets("/* unfinished", "cxx"), [0]);
}

#[test]
fn captured_go_comments_use_source_lines_and_ignore_raw_string_contents() {
    let document = serde_json::from_str(include_str!("../testdata/no-comments.cpg.json")).unwrap();
    let facts = crate::cpg::rule_facts(&document).unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(
        facts[0]
            .comments
            .iter()
            .map(|comment| (comment.line, comment.kind))
            .collect::<Vec<_>>(),
        [
            (5, CommentKind::ToolDirective),
            (6, CommentKind::Ordinary),
            (8, CommentKind::Ordinary),
            (9, CommentKind::Ordinary)
        ]
    );
}

#[test]
fn empty_blocks_and_unicode_comments_remain_ordinary() {
    assert_eq!(offsets("/**/", "rust"), [0]);
    assert_eq!(offsets("é // explain\r\n// next", "rust"), [3, 15]);
}

#[test]
fn documentation_and_directive_categories_survive_capture() {
    assert_eq!(
        scan("/// docs\n// fmt: off\n// ordinary", vec![], "rust"),
        [
            (0, CommentKind::Documentation),
            (9, CommentKind::ToolDirective),
            (21, CommentKind::Ordinary)
        ]
    );
    assert_eq!(
        scan("#!tool\n# ordinary", vec![], "python"),
        [(0, CommentKind::ToolDirective), (7, CommentKind::Ordinary)]
    );
}
