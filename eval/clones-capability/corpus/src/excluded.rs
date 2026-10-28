//! Deliberately exceeds both limits before workspace path exclusion.

pub fn over_both_limits(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
    let first = a + b;
    let second = first + c;
    let third = second + d;
    third + e
}
