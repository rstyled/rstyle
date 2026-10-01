pub fn callee(a: u8, b: u8, c: u8, d: u8, e: u8) -> u8 {
    a + b + c + d + e
}

pub fn caller(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> u8 {
    callee(a, b, c, d, e) + f
}

pub fn lonely(a: u8, b: u8, c: u8, d: u8, e: u8) -> u8 {
    a * b * c * d * e
}
