const CAP: usize = 8;

#[derive(Clone)]
pub struct Accel {
    bytes: [u8; CAP],
}

impl Accel {
    pub fn new() -> Accel {
        Accel { bytes: [0; CAP] }
    }

    pub fn from_bytes_unchecked(bytes: [u8; 4]) -> Accel {
        Accel {
            bytes: [bytes[0], bytes[1], bytes[2], bytes[3], 0, 0, 0, 0],
        }
    }

    pub fn len(&self) -> usize {
        usize::from(self.bytes[0])
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn needles(&self) -> &[u8] {
        &self.bytes[1..1 + self.len()]
    }
}

pub fn getter_len(value: &Accel) -> usize {
    value.len()
}

pub fn getter_empty(value: &Accel) -> bool {
    value.is_empty()
}

pub fn getter_view(value: &Accel) -> &[u8] {
    value.needles()
}

pub fn first(value: &Accel) -> u8 {
    value.needles()[0]
}

pub fn saved(value: &Accel, index: usize) -> u8 {
    let view = value.needles();
    view[index]
}

pub fn selected<'a>(left: &'a Accel, right: &'a Accel, pick: bool) -> &'a [u8] {
    let value = if pick { left } else { right };
    value.needles()
}

pub fn slice_window(value: &[u8], lo: usize, hi: usize) -> &[u8] {
    &value[lo..hi]
}

pub fn mutate(value: &mut Accel) {
    value.bytes[0] = 1;
}

pub fn owned(value: Accel) -> u8 {
    value.bytes[0]
}

pub fn make() -> Accel {
    Accel::new()
}

pub fn suffix(value: &Accel) -> &[u8] {
    &value.bytes[1..]
}

pub struct Arithmetic {
    bytes: [u8; CAP + 1],
}

pub fn arithmetic(value: &Arithmetic) -> u8 {
    value.bytes[0]
}
