//! Conservative lexical bounds before parsing optional consumer source.
pub(super) fn within_budget(source: &str) -> bool {
    source.len() <= 256 * 1024
        && source
            .bytes()
            .try_fold(SyntaxBudget::default(), SyntaxBudget::advance)
            .is_some()
}

#[derive(Default)]
struct SyntaxBudget {
    depth: usize,
    punctuation: usize,
    words: usize,
    in_word: bool,
}

impl SyntaxBudget {
    fn advance(self, byte: u8) -> Option<Self> {
        let word = byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii();
        let depth = match byte {
            b'{' | b'(' | b'[' => self.depth + 1,
            b'}' | b')' | b']' => self.depth.saturating_sub(1),
            _ => self.depth,
        };
        let next = Self {
            depth,
            punctuation: self.punctuation + usize::from(!word && !byte.is_ascii_whitespace()),
            words: self.words + usize::from(word && !self.in_word),
            in_word: word,
        };

        (next.depth <= 128 && next.punctuation <= 128 && next.words <= 128).then_some(next)
    }
}
