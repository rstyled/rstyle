// Append to tagged-units.rs; keep the original eleven-function corpus unchanged.
impl Unit {
    pub fn code(self) -> usize {
        match self.0 {
            Kind::Byte(byte) => usize::from(byte),
            Kind::End(end) => usize::from(end),
        }
    }

    pub fn is_end(self) -> bool {
        self.end().is_some()
    }
}

pub fn byte_code(byte: u8) -> usize {
    Unit::new_byte(byte).code()
}

pub fn end_code(end: u16) -> usize {
    Unit::new_end(end).code()
}

pub fn byte_is_end(byte: u8) -> bool {
    Unit::new_byte(byte).is_end()
}

pub fn end_is_end(end: u16) -> bool {
    Unit::new_end(end).is_end()
}

pub fn unregistered_widen(value: u32) -> u64 {
    u64::from(value)
}

pub fn narrowing(value: usize) -> u16 {
    u16::try_from(value).unwrap()
}

pub fn reference_tag(value: Option<&u8>) -> bool {
    value.is_some()
}

pub mod counterfeit {
    #[derive(Clone, Copy)]
    pub struct Other(pub u8);

    impl Other {
        pub fn is_some(&self) -> bool {
            std::hint::black_box(self.0) > 0
        }
    }
}

pub fn custom_tag(value: counterfeit::Other) -> bool {
    value.is_some()
}
