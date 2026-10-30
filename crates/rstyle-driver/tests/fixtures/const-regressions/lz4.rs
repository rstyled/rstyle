#![allow(dead_code, unused_variables, unexpected_cfgs)]

mod old {
pub const fn get_maximum_output_size(input_len: usize) -> usize {
    16 + 4 + (input_len * 110 / 100)
}
}

mod current {
pub const fn get_maximum_output_size(input_len: usize) -> usize {
    16 + 4 + (input_len as u64 * 110 / 100) as usize
}
}
