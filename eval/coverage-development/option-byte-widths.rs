// Source-selected concrete scalar Option compositions; not an active contract.
pub fn match_width(byte: u8) -> Option<usize> {
    match byte {
        0b0000_0000..=0b0111_1111 => Some(1),
        0b1000_0000..=0b1011_1111 => None,
        0b1100_0000..=0b1101_1111 => Some(2),
        0b1110_0000..=0b1110_1111 => Some(3),
        0b1111_0000..=0b1111_0111 => Some(4),
        _ => None,
    }
}

pub fn branch_width(byte: u8) -> Option<usize> {
    if byte <= 0x7F {
        return Some(1);
    } else if byte & 0b1100_0000 == 0b1000_0000 {
        return None;
    } else if byte <= 0b1101_1111 {
        Some(2)
    } else if byte <= 0b1110_1111 {
        Some(3)
    } else if byte <= 0b1111_0111 {
        Some(4)
    } else {
        None
    }
}

pub fn width_or(byte: u8, fallback: usize) -> usize {
    let width = match_width(byte);
    match width { Some(n) => n, None => fallback }
}

pub fn saved_width(mut byte: u8) -> usize {
    let width = match_width(byte);
    byte = 0;
    match width { Some(n) => n, None => byte as usize }
}

pub fn nested(byte: u8) -> Option<Option<usize>> { Some(match_width(byte)) }

mod user { pub enum Option { Some(usize), None } }
pub fn custom(byte: u8) -> user::Option {
    if byte < 128 { user::Option::Some(1) } else { user::Option::None }
}
