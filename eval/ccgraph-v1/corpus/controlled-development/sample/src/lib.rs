#![allow(dead_code, unused_variables)]
pub mod mask_base {
pub fn mask(a: u32, b: u32) -> u32 { let x = a & b; x | a }
}
pub mod mask_format {
pub fn mask(a: u32, b: u32) -> u32 { /* source-only formatting variant */ let x = a & b; x | a }
}
pub mod mask_rename {
pub fn veil(left: u32, right: u32) -> u32 { let saved = left & right; saved | left }
}
pub mod mask_semantic {
pub fn mask(a: u32, b: u32) -> u32 { let x = a ^ b; x & a }
}
pub mod assign_base {
pub fn assign(flag: bool, a: i32, b: i32) -> i32 { let mut x = a; if flag { x = b; } else { x = a; } x }
}
pub mod assign_format {
pub fn assign(flag: bool, a: i32, b: i32) -> i32 { /* source-only formatting variant */ let mut x = a; if flag { x = b; } else { x = a; } x }
}
pub mod assign_rename {
pub fn select(flag: bool, left: i32, right: i32) -> i32 { let mut saved = left; if flag { saved = right; } else { saved = left; } saved }
}
pub mod assign_semantic {
pub fn assign(flag: bool, a: i32, b: i32) -> i32 { let mut x = a; if flag { x = a; } else { x = b; } x }
}
pub mod empty_base {
pub fn empty(unused: i32) { }
}
pub mod empty_format {
pub fn empty(unused: i32) { /* source-only formatting variant */ }
}
pub mod empty_rename {
pub fn noop(ignored: i32) { }
}
pub mod empty_semantic {
pub fn empty(unused: i32) { () }
}
pub mod unsupported {
pub fn unsupported(x: u32) -> u32 { x.wrapping_add(1) }
}
pub mod unknown {
pub fn conditional(x: u32) -> u32 { #[cfg(any())] { missing(x); } x }
}
