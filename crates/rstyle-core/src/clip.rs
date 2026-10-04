//! Char-boundary-safe truncation for text shown in facts.

/// `text` when it has at most `limit` chars, otherwise its first `limit - 3` chars plus `...`.
pub fn clip(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let kept: String = text.chars().take(limit.saturating_sub(3)).collect();
    format!("{kept}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_unchanged() {
        assert_eq!(clip("alpha", 32), "alpha");
        assert_eq!(clip(&"a".repeat(32), 32).len(), 32);
    }

    #[test]
    fn long_text_keeps_limit_minus_three_and_dots() {
        let out = clip(&"a".repeat(40), 32);
        assert_eq!(out, format!("{}...", "a".repeat(29)));
    }

    #[test]
    fn cuts_on_char_boundaries() {
        let out = clip(&"é".repeat(40), 24);
        assert_eq!(out, format!("{}...", "é".repeat(21)));
    }
}
