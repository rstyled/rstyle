pub fn is_word_byte(c: u8) -> bool {
    match c {
        b'_' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' => true,
        _ => false,
    }
}

pub fn caller(byte: u8) -> bool { is_word_byte(byte) }
pub fn underscore(byte: u8) -> bool { byte == b'_' }
pub fn escaped() -> u8 { b'\n' }
pub fn exhaustive(byte: u8) -> bool {
    match byte { b'\0'..=b'\x7f' => false, b'\x80'..=b'\xff' => true }
}
pub fn mixed(byte: u8) -> bool { match byte { 0..=b'9' => true, _ => false } }
pub fn checked(bytes: &[u8], index: usize) -> bool {
    match bytes.get(index) { Some(&byte) => is_word_byte(byte), None => false }
}
pub fn saved(mut byte: u8) -> bool {
    let answer = is_word_byte(byte);
    byte = 0;
    let _ = byte;
    answer
}
pub fn as_index() -> usize { b'_' as usize }

pub fn characters(value: char) -> bool { value == 'a' }
pub fn byte_string() -> &'static [u8] { b"az" }
