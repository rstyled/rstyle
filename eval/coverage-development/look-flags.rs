// Fixed source-selected development family; original crate is measured separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Look {
    /// Match the beginning of text. Specifically, this matches at the starting
    /// position of the input.
    Start = 1 << 0,
    /// Match the end of text. Specifically, this matches at the ending
    /// position of the input.
    End = 1 << 1,
    /// Match the beginning of a line or the beginning of text. Specifically,
    /// this matches at the starting position of the input, or at the position
    /// immediately following a `\n` character.
    StartLF = 1 << 2,
    /// Match the end of a line or the end of text. Specifically, this matches
    /// at the end position of the input, or at the position immediately
    /// preceding a `\n` character.
    EndLF = 1 << 3,
    /// Match the beginning of a line or the beginning of text. Specifically,
    /// this matches at the starting position of the input, or at the position
    /// immediately following either a `\r` or `\n` character, but never after
    /// a `\r` when a `\n` follows.
    StartCRLF = 1 << 4,
    /// Match the end of a line or the end of text. Specifically, this matches
    /// at the end position of the input, or at the position immediately
    /// preceding a `\r` or `\n` character, but never before a `\n` when a `\r`
    /// precedes it.
    EndCRLF = 1 << 5,
    /// Match an ASCII-only word boundary. That is, this matches a position
    /// where the left adjacent character and right adjacent character
    /// correspond to a word and non-word or a non-word and word character.
    WordAscii = 1 << 6,
    /// Match an ASCII-only negation of a word boundary.
    WordAsciiNegate = 1 << 7,
    /// Match a Unicode-aware word boundary. That is, this matches a position
    /// where the left adjacent character and right adjacent character
    /// correspond to a word and non-word or a non-word and word character.
    WordUnicode = 1 << 8,
    /// Match a Unicode-aware negation of a word boundary.
    WordUnicodeNegate = 1 << 9,
    /// Match the start of an ASCII-only word boundary. That is, this matches a
    /// position at either the beginning of the haystack or where the previous
    /// character is not a word character and the following character is a word
    /// character.
    WordStartAscii = 1 << 10,
    /// Match the end of an ASCII-only word boundary. That is, this matches
    /// a position at either the end of the haystack or where the previous
    /// character is a word character and the following character is not a word
    /// character.
    WordEndAscii = 1 << 11,
    /// Match the start of a Unicode word boundary. That is, this matches a
    /// position at either the beginning of the haystack or where the previous
    /// character is not a word character and the following character is a word
    /// character.
    WordStartUnicode = 1 << 12,
    /// Match the end of a Unicode word boundary. That is, this matches a
    /// position at either the end of the haystack or where the previous
    /// character is a word character and the following character is not a word
    /// character.
    WordEndUnicode = 1 << 13,
    /// Match the start half of an ASCII-only word boundary. That is, this
    /// matches a position at either the beginning of the haystack or where the
    /// previous character is not a word character.
    WordStartHalfAscii = 1 << 14,
    /// Match the end half of an ASCII-only word boundary. That is, this
    /// matches a position at either the end of the haystack or where the
    /// following character is not a word character.
    WordEndHalfAscii = 1 << 15,
    /// Match the start half of a Unicode word boundary. That is, this matches
    /// a position at either the beginning of the haystack or where the
    /// previous character is not a word character.
    WordStartHalfUnicode = 1 << 16,
    /// Match the end half of a Unicode word boundary. That is, this matches
    /// a position at either the end of the haystack or where the following
    /// character is not a word character.
    WordEndHalfUnicode = 1 << 17,
}
impl Look {
pub const fn as_repr(self) -> u32 {
        // AFAIK, 'as' is the only way to zero-cost convert an int enum to an
        // actual int.
        self as u32
    }
}
#[derive(Clone, Copy)]
pub struct LookSet { bits: u32 }
impl LookSet {
pub const fn empty() -> LookSet { LookSet { bits: 0 } }
pub fn contains(self, look: Look) -> bool {
        self.bits & look.as_repr() != 0
    }
pub fn contains_anchor(&self) -> bool {
        self.contains_anchor_haystack() || self.contains_anchor_line()
    }
pub fn contains_anchor_haystack(&self) -> bool {
        self.contains(Look::Start) || self.contains(Look::End)
    }
pub fn contains_anchor_line(&self) -> bool {
        self.contains(Look::StartLF)
            || self.contains(Look::EndLF)
            || self.contains(Look::StartCRLF)
            || self.contains(Look::EndCRLF)
    }
pub fn contains_anchor_lf(&self) -> bool {
        self.contains(Look::StartLF) || self.contains(Look::EndLF)
    }
pub fn contains_anchor_crlf(&self) -> bool {
        self.contains(Look::StartCRLF) || self.contains(Look::EndCRLF)
    }
pub fn contains_word(self) -> bool {
        self.contains_word_unicode() || self.contains_word_ascii()
    }
pub fn contains_word_unicode(self) -> bool {
        self.contains(Look::WordUnicode)
            || self.contains(Look::WordUnicodeNegate)
            || self.contains(Look::WordStartUnicode)
            || self.contains(Look::WordEndUnicode)
            || self.contains(Look::WordStartHalfUnicode)
            || self.contains(Look::WordEndHalfUnicode)
    }
pub fn contains_word_ascii(self) -> bool {
        self.contains(Look::WordAscii)
            || self.contains(Look::WordAsciiNegate)
            || self.contains(Look::WordStartAscii)
            || self.contains(Look::WordEndAscii)
            || self.contains(Look::WordStartHalfAscii)
            || self.contains(Look::WordEndHalfAscii)
    }
pub fn singleton(look: Look) -> LookSet {
        LookSet::empty().insert(look)
    }
pub fn insert(self, look: Look) -> LookSet {
        LookSet { bits: self.bits | look.as_repr() }
    }
pub fn remove(self, look: Look) -> LookSet {
        LookSet { bits: self.bits & !look.as_repr() }
    }
}
