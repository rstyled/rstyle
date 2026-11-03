//! Source spans without the parentheses around an expression: rustc gives a
//! parenthesized expression the parentheses' span, while its location is its own.

use rustc_middle::ty::TyCtxt;
use rustc_span::{BytePos, Span};

/// Byte range of `content[start..end]` without enclosing parentheses and the space
/// inside them.
pub fn unparenthesized(content: &str, mut start: usize, mut end: usize) -> (usize, usize) {
    let bytes = content.as_bytes();
    while end >= start + 2
        && bytes.get(start) == Some(&b'(')
        && bytes.get(end - 1) == Some(&b')')
        && closing(bytes, start) == Some(end - 1)
    {
        let inner = &content[start + 1..end - 1];
        let leading = inner.len() - inner.trim_start().len();
        let trailing = inner.len() - inner.trim_end().len();
        (start, end) = (start + 1 + leading, end - 1 - trailing);
    }
    (start, end)
}

/// Where an expression at `span` begins, past enclosing parentheses.
pub fn start(tcx: TyCtxt<'_>, span: Span) -> BytePos {
    let file = tcx.sess.source_map().lookup_source_file(span.lo());
    let Some(content) = file.src.as_deref() else {
        return span.lo();
    };
    let offset = |position: BytePos| (position - file.start_pos).0 as usize;
    if span.hi() > file.end_position() {
        return span.lo();
    }
    let (start, _) = unparenthesized(content, offset(span.lo()), offset(span.hi()));

    file.start_pos + BytePos(start as u32)
}

fn closing(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut index = open;
    while index < bytes.len() {
        match (bytes[index], in_string) {
            (b'\\', true) => index += 1,
            (b'"', _) => in_string = !in_string,
            (b'(', false) => depth += 1,
            (b')', false) => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }

    None
}
