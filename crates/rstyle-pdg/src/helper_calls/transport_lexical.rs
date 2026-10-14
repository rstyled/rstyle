//! Preserve ordered JSON document bytes without object reserialization.

use serde_json::Value;

pub(super) fn fragments(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut fragments = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'"' {
            cursor += 1;
            continue;
        }
        if start != cursor {
            fragments.push(&text[start..cursor]);
        }
        start = cursor;
        cursor = quoted_end(bytes, cursor);
        fragments.push(&text[start..cursor]);
        start = cursor;
    }
    if start != bytes.len() {
        fragments.push(&text[start..]);
    }

    fragments
}

fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor += 2,
            b'"' => return cursor + 1,
            _ => cursor += 1,
        }
    }

    cursor
}

pub(super) fn quoted(fragment: &str) -> Option<String> {
    if !fragment.starts_with('"') {
        return None;
    }
    let decoded: String = serde_json::from_str(fragment).ok()?;
    (serde_json::to_string(&decoded).ok()? == fragment).then_some(decoded)
}

pub(super) fn string(value: Value) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text),
        _ => Err("Lexical helper transport requires string fragments".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_valid_json_at_exact_tokens_without_reordering_or_unescaping() {
        let text = "{\"z\":\"a\\\"b\\\\c\", \"a\":\"\\u0061\"}\r\n";
        assert!(serde_json::from_str::<Value>(text).is_ok());
        assert_eq!(fragments(text).concat(), text);
        assert_eq!(quoted("\"a\\\"b\\\\c\"").as_deref(), Some("a\"b\\c"));
        assert_eq!(quoted("\"\\u0061\""), None);
    }
}
