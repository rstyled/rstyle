#![allow(dead_code, unused_variables, unexpected_cfgs)]

mod old17 {
pub const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 25 != 0 || year % 16 == 0)
}
}

mod old36 {
pub const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 25 != 0 || year % 16 == 0)
}
}

mod current {
pub const fn is_leap_year(year: i32) -> bool {
    (year as i64)
        .unsigned_abs()
        .wrapping_mul(0x4000_0000_28F5_C28F)
        & 0xC000_000F_8000_000F
        <= 0xF_8000_0000
}
}
