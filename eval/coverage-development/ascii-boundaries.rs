// Source-first ASCII boundary development selection. Natural capture uses the complete original crate.
#[derive(Clone, Copy)]
pub struct Byte(u8);
pub struct Matcher { lineterm: Byte }
impl Matcher {
pub fn get_line_terminator(&self) -> u8 {
        self.lineterm.0
    }

pub fn is_word_ascii(&self, haystack: &[u8], at: usize) -> bool {
        let word_before = at > 0 && utf8::is_word_byte(haystack[at - 1]);
        let word_after =
            at < haystack.len() && utf8::is_word_byte(haystack[at]);
        word_before != word_after
    }

pub fn is_word_ascii_negate(&self, haystack: &[u8], at: usize) -> bool {
        !self.is_word_ascii(haystack, at)
    }

pub fn is_word_start_ascii(&self, haystack: &[u8], at: usize) -> bool {
        let word_before = at > 0 && utf8::is_word_byte(haystack[at - 1]);
        let word_after =
            at < haystack.len() && utf8::is_word_byte(haystack[at]);
        !word_before && word_after
    }

pub fn is_word_end_ascii(&self, haystack: &[u8], at: usize) -> bool {
        let word_before = at > 0 && utf8::is_word_byte(haystack[at - 1]);
        let word_after =
            at < haystack.len() && utf8::is_word_byte(haystack[at]);
        word_before && !word_after
    }

pub fn is_word_start_half_ascii(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> bool {
        let word_before = at > 0 && utf8::is_word_byte(haystack[at - 1]);
        !word_before
    }

pub fn is_word_end_half_ascii(&self, haystack: &[u8], at: usize) -> bool {
        let word_after =
            at < haystack.len() && utf8::is_word_byte(haystack[at]);
        !word_after
    }
}
pub mod utf8 {
    pub fn is_word_byte(byte: u8) -> bool {
        match byte { b'0'..=b'9' | b'A'..=b'Z' | b'_' | b'a'..=b'z' => true, _ => false }
    }
}
pub fn getter(value: &Matcher) -> u8 { value.get_line_terminator() }
pub fn repeated(value: &Matcher, bytes: &[u8], at: usize) -> bool {
    value.is_word_ascii(bytes, at) != value.is_word_ascii_negate(bytes, at)
}
pub fn construct(byte: u8) -> Matcher { Matcher { lineterm: Byte(byte) } }
pub fn consume(value: Matcher) -> u8 { value.lineterm.0 }
pub fn write(value: &mut Matcher, byte: u8) { value.lineterm.0 = byte; }
pub struct NonCopy { byte: u8 }
pub struct NestedNonCopy { inner: NonCopy }
pub fn noncopy(value: &NestedNonCopy) -> u8 { value.inner.byte }
pub struct Dropping { byte: u8 }
impl Drop for Dropping { fn drop(&mut self) { self.byte = 0; } }
pub fn dropping(value: &Dropping) -> u8 { value.byte }
static EFFECT: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
pub fn effectful(byte: u8) -> bool {
    EFFECT.store(byte, core::sync::atomic::Ordering::Relaxed); byte == b'_'
}
pub fn through_effect(bytes: &[u8], at: usize) -> bool {
    at < bytes.len() && effectful(bytes[at])
}
