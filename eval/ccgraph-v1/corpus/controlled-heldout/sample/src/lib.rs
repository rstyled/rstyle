#![allow(dead_code, unused_variables)]
pub mod shadow_base {
pub fn shadow(a: u32, b: u32) -> u32 { let a = a & b; let a = a | b; a }
}
pub mod shadow_format {
pub fn shadow(a: u32, b: u32) -> u32 { /* source-only formatting variant */ let a = a & b; let a = a | b; a }
}
pub mod shadow_rename {
pub fn layers(left: u32, right: u32) -> u32 { let left = left & right; let left = left | right; left }
}
pub mod shadow_semantic {
pub fn shadow(a: u32, b: u32) -> u32 { let a = a ^ b; let a = a & b; a }
}
pub mod short_base {
pub fn short(a: bool, b: bool, c: bool) -> bool { a && (b || c) }
}
pub mod short_format {
pub fn short(a: bool, b: bool, c: bool) -> bool { /* source-only formatting variant */ a && (b || c) }
}
pub mod short_rename {
pub fn decide(x: bool, y: bool, z: bool) -> bool { x && (y || z) }
}
pub mod short_semantic {
pub fn short(a: bool, b: bool, c: bool) -> bool { a || (b && c) }
}
pub mod merge_base {
pub fn merge(flag: bool, a: i32, b: i32) -> i32 { let x = if flag { a } else { b }; x }
}
pub mod merge_format {
pub fn merge(flag: bool, a: i32, b: i32) -> i32 { /* source-only formatting variant */ let x = if flag { a } else { b }; x }
}
pub mod merge_rename {
pub fn choose(test: bool, left: i32, right: i32) -> i32 { let result = if test { left } else { right }; result }
}
pub mod merge_semantic {
pub fn merge(flag: bool, a: i32, b: i32) -> i32 { let x = if flag { b } else { a }; x }
}
pub mod unsupported {
pub fn unsupported(x: u32) -> u32 { x.wrapping_add(1) }
}
pub mod unknown {
pub fn conditional(x: u32) -> u32 { #[cfg(any())] { missing(x); } x }
}
