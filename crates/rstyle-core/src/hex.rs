//! Names written as lowercase hexadecimal: content digests and candidate ids.

/// Whether `text` is exactly `digits` lowercase hexadecimal digits, and so can never be a path.
pub fn is_lowercase(text: &str, digits: usize) -> bool {
    text.len() == digits
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_the_digits_asked_for_in_lowercase_are_hex() {
        assert!(super::is_lowercase("0a9f", 4));
        assert!(
            !super::is_lowercase("0A9F", 4),
            "uppercase is another spelling"
        );
        assert!(!super::is_lowercase("0a9", 4));
        assert!(!super::is_lowercase("../x", 4));
    }
}
