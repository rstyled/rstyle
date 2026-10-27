const CAP: usize = 8;

#[derive(Clone)]
pub struct Accel {
    bytes: [u8; CAP],
}

impl Accel {
    pub fn len(&self) -> usize {
        usize::from(self.bytes[0])
    }

    pub fn needles(&self) -> &[u8] {
        &self.bytes[1..1 + self.len()]
    }

    pub fn contains(&self, byte: u8) -> bool {
        self.needles().iter().position(|&b| b == byte).is_some()
    }
}

pub fn slice_contains(bytes: &[u8], byte: u8) -> bool {
    bytes.iter().position(|&b| b == byte).is_some()
}

pub fn caller(value: &Accel, byte: u8) -> bool {
    value.contains(byte)
}

pub fn saved(a: &Accel, b: &Accel, byte: u8) -> bool {
    let mut selected = a;
    let view = selected.needles();
    selected = b;
    let _ = selected;
    view.iter().position(|&b| b == byte).is_some()
}

pub fn selected(a: &Accel, b: &Accel, pick: bool, byte: u8) -> bool {
    let value = if pick { a } else { b };
    value.contains(byte)
}

pub fn window(bytes: &[u8], lo: usize, hi: usize, byte: u8) -> bool {
    bytes[lo..hi].iter().position(|&b| b == byte).is_some()
}

pub fn first_position(bytes: &[u8], byte: u8) -> Option<usize> {
    bytes.iter().position(|&b| b == byte)
}

pub fn saved_position(bytes: &[u8], byte: u8) -> bool {
    let found = bytes.iter().position(|&b| b == byte);
    found.is_some()
}

pub fn position_caller(bytes: &[u8], byte: u8) -> Option<usize> {
    first_position(bytes, byte)
}

pub fn owned(value: Accel, byte: u8) -> bool {
    value.contains(byte)
}

pub fn mutable(bytes: &mut [u8], byte: u8) -> bool {
    bytes.iter_mut().position(|b| *b == byte).is_some()
}

pub fn effectful(bytes: &[u8], byte: u8) -> bool {
    let mut visited = false;
    bytes
        .iter()
        .position(|&b| {
            visited = true;
            b == byte
        })
        .is_some()
}

pub fn saved_iterator(bytes: &[u8], byte: u8) -> bool {
    let mut iterator = bytes.iter();
    let _ = iterator.next();
    iterator.position(|&b| b == byte).is_some()
}

pub fn move_capture(bytes: &[u8], byte: u8) -> bool {
    bytes.iter().position(move |&b| b == byte).is_some()
}
