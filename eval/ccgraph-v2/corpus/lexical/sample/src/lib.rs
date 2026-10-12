#![allow(dead_code, unused_variables, unexpected_cfgs)]

mod max_step_2 {
const fn max_step_2(bits: usize, is_signed: bool) -> usize {
    match bits {
        8 if is_signed => 7,
        8 if !is_signed => 8,
        16 if is_signed => 15,
        16 if !is_signed => 16,
        32 if is_signed => 31,
        32 if !is_signed => 32,
        64 if is_signed => 63,
        64 if !is_signed => 64,
        128 if is_signed => 127,
        128 if !is_signed => 128,
        _ => 1,
    }
}
}

mod min_step_2 {
const fn min_step_2(bits: usize, is_signed: bool) -> usize {
    match bits {
        8 if is_signed => 7,
        8 if !is_signed => 8,
        16 if is_signed => 15,
        16 if !is_signed => 16,
        32 if is_signed => 31,
        32 if !is_signed => 32,
        64 if is_signed => 63,
        64 if !is_signed => 64,
        128 if is_signed => 127,
        128 if !is_signed => 128,
        _ => 1,
    }
}
}
