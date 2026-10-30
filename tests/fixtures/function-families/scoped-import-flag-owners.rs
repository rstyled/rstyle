#[macro_use]
mod macros;
mod exports;
pub use crate::exports::*;

#[derive(Clone, Copy)]
pub struct Flags(u64);

impl Flags {
    pub fn empty() -> Flags { Flags(0) }
    pub fn is_empty(self) -> bool { self.0 == 0 }
}

pub fn empty_flags() -> Flags { Flags::empty() }
pub fn roundtrip() -> bool { Flags::empty().is_empty() }
pub fn test_bits(bits: u64) -> bool { Flags(bits).is_empty() }
