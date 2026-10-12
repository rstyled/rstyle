#![allow(dead_code, unused_variables, unexpected_cfgs)]

mod scalar {
pub fn scalar(x: u8) -> u8 { x & 15 }
}

mod unit {
pub fn unit(unused: i32) {}
}

mod generic {
pub fn generic<T>(x: T) -> T { x }
}

mod r#macro {
pub fn macro_value(x: i32) -> i32 { dbg!(x) }
}

mod conditional {
pub fn conditional(x: i32) -> i32 { #[cfg(any())] let removed = 7; x }
}

mod conditional_bool {
pub fn conditional_bool(x: bool) -> bool { #[cfg(any())] let removed = false; !x }
}
