//! Comment categories in captured function source. Literal spans come from the compiler,
//! so raw strings, escapes and comment markers in strings need no second language lexer.
//! How a language writes its comments is data, one file per frontend namespace under
//! `contracts/comments`, so a language needs a file here and no code.

use super::model::{Function, NodeKind};
use super::source::Sources;
use crate::facts::{CommentKind, SourceComment};
use crate::versioned::ByNamespace;
use serde::Deserialize;
use std::ops::Range;

const SCHEMA: &str = "rstyle.comment-syntax/1";
const EMBEDDED: &[&str] = &[
    include_str!("contracts/comments/rust.toml"),
    include_str!("contracts/comments/go.toml"),
    include_str!("contracts/comments/cxx.toml"),
    include_str!("contracts/comments/py.toml"),
];

/// How one namespace writes comments. A namespace without a file has none, so nothing in its
/// functions reads as one.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommentSyntax {
    schema: String,
    namespace: String,
    /// What starts a comment that runs to the end of its line.
    line: Vec<String>,
    #[serde(default)]
    block: Option<Block>,
    /// Openings of a comment that documents, matched from its first character.
    documentation: Vec<String>,
    /// Openings that read as ordinary though they start like documentation: `////`.
    ordinary: Vec<String>,
    /// What a tool's comment starts with, after its opening and any space.
    directives: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    open: String,
    close: String,
    /// Whether a block opened inside a block needs its own close, as Rust's do.
    nested: bool,
}

impl CommentSyntax {
    pub fn of(namespace: &str) -> &'static CommentSyntax {
        static SYNTAX: ByNamespace<CommentSyntax> = ByNamespace::empty();
        SYNTAX.of(namespace, (EMBEDDED, parse))
    }

    /// The end and category of a comment at the start of `rest`, `at` into `source`.
    fn comment_at(&self, source: &str, at: usize) -> Option<(usize, CommentKind)> {
        let rest = &source[at..];
        let (opening, end) = match self
            .block
            .as_ref()
            .filter(|block| rest.starts_with(&block.open))
        {
            Some(block) => (
                &block.open,
                self.block_end(source, at + block.open.len(), block),
            ),
            None => {
                let opening = self
                    .line
                    .iter()
                    .find(|marker| rest.starts_with(marker.as_str()))?;
                (
                    opening,
                    rest.find('\n').map_or(source.len(), |end| at + end),
                )
            }
        };
        let comment = &source[at..end];
        Some((end, self.kind(comment, &comment[opening.len()..])))
    }

    fn kind(&self, comment: &str, text: &str) -> CommentKind {
        let starts = |openings: &[String]| {
            openings
                .iter()
                .any(|opening| comment.starts_with(opening.as_str()))
        };
        let text = text.trim_start();
        if starts(&self.documentation) && !starts(&self.ordinary) {
            CommentKind::Documentation
        } else if self
            .directives
            .iter()
            .any(|prefix| text.starts_with(prefix.as_str()))
        {
            CommentKind::ToolDirective
        } else {
            CommentKind::Ordinary
        }
    }

    fn block_end(&self, source: &str, mut at: usize, block: &Block) -> usize {
        let mut depth = 1;
        while at < source.len() {
            let rest = &source[at..];
            if rest.starts_with(&block.close) {
                depth -= 1;
                at += block.close.len();
                if depth == 0 {
                    return at;
                }
            } else if block.nested && rest.starts_with(&block.open) {
                depth += 1;
                at += block.open.len();
            } else {
                at += rest.chars().next().map_or(1, char::len_utf8);
            }
        }
        at
    }
}

fn parse(text: &str) -> Result<(String, CommentSyntax), String> {
    let syntax: CommentSyntax = crate::versioned::parse(
        text,
        (SCHEMA, "comment syntax"),
        |syntax: &CommentSyntax| &syntax.schema,
    )?;
    Ok((syntax.namespace.clone(), syntax))
}

pub fn lines(
    function: &Function,
    sources: &Sources<'_>,
    syntax: &CommentSyntax,
) -> Result<Vec<SourceComment>, String> {
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
    Ok(scan(source, literals, syntax)
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

fn scan(
    source: &str,
    literals: Vec<Range<usize>>,
    syntax: &CommentSyntax,
) -> Vec<(usize, CommentKind)> {
    let mut slot = 0;
    let mut found = Vec::new();
    let mut at = 0;
    while at < source.len() {
        while literals.get(slot).is_some_and(|span| span.end <= at) {
            slot += 1;
        }
        let next = if let Some(span) = literals.get(slot).filter(|span| span.start <= at) {
            span.end
        } else if let Some((end, kind)) = syntax.comment_at(source, at) {
            found.push((at, kind));
            end
        } else {
            at + source[at..].chars().next().map_or(1, char::len_utf8)
        };
        at = next;
    }
    found
}

#[cfg(test)]
mod tests;
