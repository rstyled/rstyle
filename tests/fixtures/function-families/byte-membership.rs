// Intended positives: constructor, getter, empty/word/saved-owner callers.
// Partial leaves remain visible; their callers must remain excluded.
#[derive(Clone, Copy)]
pub struct ByteSet { bits: BitSet }
#[derive(Clone, Copy)]
struct BitSet([u128; 2]);
enum Neighbor { A, B }

impl ByteSet {
    pub fn empty() -> Self { ByteSet { bits: BitSet([0; 2]) } }
    pub fn contains(&self, byte: u8) -> bool {
        let bucket = byte / 128;
        let bit = byte % 128;
        self.bits.0[usize::from(bucket)] & (1 << bit) > 0
    }
    pub fn unbounded(&self, index: usize) -> u128 { self.bits.0[index] }
}

pub fn empty_contains(byte: u8) -> bool { ByteSet::empty().contains(byte) }
pub fn contains_words(low: u128, high: u128, byte: u8) -> bool {
    ByteSet { bits: BitSet([low, high]) }.contains(byte)
}
pub fn contains_saved(low: u128, high: u128, byte: u8) -> bool {
    let owner = ByteSet { bits: BitSet([low, high]) };
    owner.contains(byte)
}
pub fn unbounded_caller(low: u128, high: u128, index: usize) -> u128 {
    ByteSet { bits: BitSet([low, high]) }.unbounded(index)
}
pub fn rebound(byte: u8, other: u8) -> u128 {
    let mut bit = byte % 128;
    bit = other;
    1u128 << bit
}
pub fn merged(byte: u8, other: u8, pick: bool) -> u128 {
    let bit = if pick { byte % 128 } else { other };
    1u128 << bit
}
pub fn narrower(byte: u8) -> u64 { 1u64 << (byte % 128) }
pub fn remainder(byte: u8, divisor: u8) -> u128 { 1u128 << (byte % divisor) }
pub fn narrower_caller(byte: u8) -> u64 { narrower(byte) }
pub fn remainder_caller(byte: u8, divisor: u8) -> u128 { remainder(byte, divisor) }
