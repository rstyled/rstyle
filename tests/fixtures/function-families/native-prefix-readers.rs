// Source-selected prefix/native readers; declared before admission assessment.
use core::mem::size_of;

pub fn read_u16(slice: &[u8]) -> u16 {
    let bytes: [u8; 2] = slice[..size_of::<u16>()].try_into().unwrap();
    u16::from_ne_bytes(bytes)
}
pub fn read_u32(slice: &[u8]) -> u32 {
    let bytes: [u8; 4] = slice[..size_of::<u32>()].try_into().unwrap();
    u32::from_ne_bytes(bytes)
}
pub fn read_u128(slice: &[u8]) -> u128 {
    let bytes: [u8; 16] = slice[..size_of::<u128>()].try_into().unwrap();
    u128::from_ne_bytes(bytes)
}
pub fn native_u16(bytes: [u8; 2]) -> u16 { u16::from_ne_bytes(bytes) }
pub fn choose<'a>(a: &'a [u8], b: &'a [u8], pick: bool) -> &'a [u8] {
    if pick { a } else { b }
}
pub fn selected_u16(a: &[u8], b: &[u8], pick: bool) -> u16 {
    let view = &choose(a, b, pick)[..2];
    u16::from_ne_bytes(view.try_into().unwrap())
}
pub fn saved_u16<'a>(mut a: &'a [u8], b: &'a [u8]) -> u16 {
    let view = &a[..2]; a = b;
    let bytes: [u8; 2] = view.try_into().unwrap();
    u16::from_ne_bytes(bytes)
}
pub fn snapshot_u16<'a>(a: &'a [u8], b: &'a [u8]) -> u16 {
    let mut selected = a;
    let bytes: [u8; 2] = selected[..{ selected = b; 2 }].try_into().unwrap();
    u16::from_ne_bytes(bytes)
}
// Retained negatives: suffix view, signed conversion, and a partial-flow callee.
pub fn suffix(a: &[u8], n: usize) -> &[u8] { &a[n..] }
pub fn signed(bytes: [u8; 2]) -> i16 { i16::from_ne_bytes(bytes) }
pub fn caller_u16(a: &[u8]) -> u16 { read_u16(a) }
