// Separate development composition cases, specified before checked-access support.

pub fn checked_byte(bytes: &[u8], index: usize) -> u8 {
    match bytes.get(index) {
        Some(&byte) => byte,
        None => 0,
    }
}

pub fn helper_boundary(bytes: &[u8], index: usize) -> bool {
    is_boundary(bytes, index)
}

pub fn saved_byte<'a>(first: &'a [u8], second: &'a [u8], index: usize) -> u8 {
    let mut selected = first;
    let saved = selected.get(index);
    selected = second;
    match saved {
        Some(&byte) => byte,
        None => selected.len() as u8,
    }
}

pub fn mutable_byte(bytes: &mut [u8], index: usize) -> u8 {
    match bytes.get_mut(index) {
        Some(byte) => {
            *byte = 42;
            *byte
        }
        None => 0,
    }
}
