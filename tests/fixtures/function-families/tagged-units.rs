#[derive(Clone, Copy)]
pub enum Kind {
    Byte(u8),
    End(u16),
}

#[derive(Clone, Copy)]
pub struct Unit(Kind);

impl Unit {
    pub fn new_byte(byte: u8) -> Unit {
        Unit(Kind::Byte(byte))
    }

    pub fn new_end(end: u16) -> Unit {
        Unit(Kind::End(end))
    }

    pub fn byte(self) -> Option<u8> {
        match self.0 {
            Kind::Byte(byte) => Some(byte),
            Kind::End(_) => None,
        }
    }

    pub fn end(self) -> Option<u16> {
        match self.0 {
            Kind::Byte(_) => None,
            Kind::End(end) => Some(end),
        }
    }
}

pub fn roundtrip(byte: u8) -> Option<u8> {
    Unit::new_byte(byte).byte()
}

pub fn saved(byte: u8) -> Option<u8> {
    let mut selected = Unit::new_byte(byte);
    let result = selected.byte();
    selected = Unit::new_end(256);
    let _ = selected;
    result
}

pub fn nested(byte: u8) -> Option<Unit> {
    Some(Unit::new_byte(byte))
}

pub fn reference_payload(bytes: &[u8]) -> Option<&[u8]> {
    Some(bytes)
}

pub fn residual(value: Option<u8>) -> Option<u8> {
    Some(value?)
}
