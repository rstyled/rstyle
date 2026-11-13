//! Comment categories in captured function source. Literal spans come from the compiler,
//! so raw strings, escapes and comment markers in strings need no second language lexer.

use super::model::{Function, NodeKind};
use super::source::Sources;
use crate::facts::{CommentKind, SourceComment};
use std::ops::Range;

const DIRECTIVES: &[&str] = &[
    "go:",
    "line ",
    "rstyle-allow(",
    "nolint",
    "lint:",
    "NOLINT",
    "noinspection",
    "type:",
    "noqa",
    "pyright:",
    "mypy:",
    "pylint:",
    "isort:",
    "yapf:",
    "fmt:",
    "rustfmt::",
    "clang-format",
    "!",
];

pub fn lines(
    function: &Function,
    sources: &Sources<'_>,
    language: &str,
) -> Result<Vec<SourceComment>, String> {
    if !matches!(
        language,
        "rust" | "go" | "cxx" | "cpp" | "c++" | "py" | "python"
    ) {
        return Ok(Vec::new());
    }
    let span = &function.signature.span;
    let source = sources.between(span, &function.span)?;
    let mut literals: Vec<_> = function
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                NodeKind::Literal
                    | NodeKind::PatternLiteral
                    | NodeKind::Closure
                    | NodeKind::ItemDeclaration
            )
        })
        .filter_map(|node| node.span.as_ref())
        .filter(|literal| {
            literal.file == span.file
                && literal.start >= span.start
                && literal.end <= function.span.end
        })
        .map(|literal| literal.start - span.start..literal.end - span.start)
        .collect();
    literals.sort_by_key(|span| span.start);
    let first_line = sources.location(span)?.0;
    Ok(scan(source, literals, language)
        .into_iter()
        .map(|(offset, kind)| SourceComment {
            line: first_line
                + source[..offset]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count(),
            kind,
        })
        .collect())
}

fn scan(source: &str, literals: Vec<Range<usize>>, language: &str) -> Vec<(usize, CommentKind)> {
    let mut slot = 0;
    let python = matches!(language, "py" | "python");
    let mut found = Vec::new();
    let mut at = 0;
    while at < source.len() {
        while literals.get(slot).is_some_and(|span| span.end <= at) {
            slot += 1;
        }
        let next = if let Some(span) = literals.get(slot).filter(|span| span.start <= at) {
            span.end
        } else if let Some((end, kind)) = comment_at(source, at, python, language == "rust") {
            found.push((at, kind));
            end
        } else {
            at + 1
        };
        at = next;
    }
    found
}

/// End and syntax category of a comment at this offset.
fn comment_at(source: &str, at: usize, python: bool, nested: bool) -> Option<(usize, CommentKind)> {
    let rest = &source.as_bytes()[at..];
    let width = if python && rest.starts_with(b"#") {
        1
    } else if !python && (rest.starts_with(b"//") || rest.starts_with(b"/*")) {
        2
    } else {
        return None;
    };
    let block = rest.starts_with(b"/*");
    let end = if block {
        block_end(source, at + width, nested)
    } else {
        source[at..].find('\n').map_or(source.len(), |end| at + end)
    };
    let text = &source[at + width..end];
    let kind = if documentation(text, block, python) {
        CommentKind::Documentation
    } else if directive(text) {
        CommentKind::ToolDirective
    } else {
        CommentKind::Ordinary
    };
    Some((end, kind))
}

fn documentation(text: &str, block: bool, python: bool) -> bool {
    if python || (block && text == "*/") {
        return false;
    }
    if text.starts_with('!') {
        return true;
    }
    let marker = if block { '*' } else { '/' };
    text.starts_with(marker) && !text[1..].starts_with(marker)
}

fn directive(text: &str) -> bool {
    let text = text.trim_start();
    DIRECTIVES.iter().any(|prefix| text.starts_with(prefix))
}

fn block_end(source: &str, mut at: usize, nested: bool) -> usize {
    let bytes = source.as_bytes();
    let mut depth = 1;
    while at < bytes.len() {
        let rest = &bytes[at..];
        if rest.starts_with(b"*/") {
            depth -= 1;
            if depth == 0 {
                return at + 2;
            }
            at += 2;
        } else if nested && rest.starts_with(b"/*") {
            depth += 1;
            at += 2;
        } else {
            at += 1;
        }
    }
    at
}

#[cfg(test)]
mod tests;
