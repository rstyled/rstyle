// Fixed source-first development selection. Keep original corpus identities unchanged.
#[derive(Clone, Copy)]
pub enum Kind { Byte(u8), End(u16) }
#[derive(Clone, Copy)]
pub struct Unit(Kind);

impl Unit {
    pub fn new_byte(byte: u8) -> Unit { Unit(Kind::Byte(byte)) }
    pub fn new_end(end: u16) -> Unit { Unit(Kind::End(end)) }
    pub fn byte(self) -> Option<u8> {
        match self.0 { Kind::Byte(byte) => Some(byte), Kind::End(_) => None }
    }
    pub fn is_word_byte(self) -> bool { self.byte().map_or(false, table::is_word_byte) }
}

pub mod table {
pub(crate) fn is_word_byte(b: u8) -> bool {
    const fn mkwordset() -> [bool; 256] {
        // FIXME: Use as_usize() once const functions in traits are stable.
        let mut set = [false; 256];
        set[b'_' as usize] = true;

        let mut byte = b'0';
        while byte <= b'9' {
            set[byte as usize] = true;
            byte += 1;
        }
        byte = b'A';
        while byte <= b'Z' {
            set[byte as usize] = true;
            byte += 1;
        }
        byte = b'a';
        while byte <= b'z' {
            set[byte as usize] = true;
            byte += 1;
        }
        set
    }
    const WORD: [bool; 256] = mkwordset();
    WORD[b as usize]
}
}

pub fn word_byte(byte: u8) -> bool { table::is_word_byte(byte) }
pub fn word_unit(byte: u8) -> bool { Unit::new_byte(byte).is_word_byte() }
pub fn saved(value: Option<u8>, fallback: bool) -> bool {
    let answer = value.map_or(fallback, table::is_word_byte);
    answer
}
pub fn empty_default(fallback: bool) -> bool {
    let value: Option<u8> = None;
    value.map_or(fallback, table::is_word_byte)
}

pub fn scalar(byte: u8) -> bool { byte == b'_' }
pub fn named(value: Option<u8>, fallback: bool) -> bool { value.map_or(fallback, scalar) }
pub fn named_none(fallback: bool) -> bool {
    let value: Option<u8> = None;
    value.map_or(fallback, scalar)
}
use table::is_word_byte as classify;
pub fn aliased(value: Option<u8>) -> bool { value.map_or(false, classify) }

pub fn indirect(value: Option<u8>, callback: fn(u8) -> bool) -> bool {
    value.map_or(false, callback)
}
pub fn mutable(index: usize) -> bool {
    let mut table = [false; 256];
    table[index] = true;
    table[index]
}
pub fn effectful_callback(byte: u8) -> bool { core::hint::black_box(byte) == 0 }
pub fn through_effectful(value: Option<u8>) -> bool { value.map_or(false, effectful_callback) }
