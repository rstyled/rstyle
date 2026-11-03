//! Source spans without the parentheses around an expression: rustc gives a
//! parenthesized expression the parentheses' span, while its location is its own.

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
