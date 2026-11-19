pub fn borrowed(items: &[u8; 4]) {
    for _item in items {}
}

pub fn iter(items: &[u8; 4]) {
    for _item in items.iter() {}
}

pub fn chain(items: &[u8; 4]) -> usize {
    items.iter().filter(|x| **x > 0).count()
}

pub fn indexed(items: &[u8; 4]) {
    for _index in 0..items.len() {}
}

pub fn callback_loops(items: &[u8; 4]) {
    items.iter().for_each(|_| {
        for _index in 0..3usize {}
    });
}

pub fn owned(items: [u8; 4]) {
    for _item in items {}
}

pub fn const_generic<const N: usize>(items: &[u8; N]) {
    for _item in items {}
}

pub fn writable(items: &mut [u8; 4]) {
    for _item in items {}
}

pub fn computed_receiver() -> [u8; 4] {
    loop {}
}

pub fn computed_length() {
    for _index in 0..computed_receiver().len() {}
}

pub fn computed_read() -> usize {
    computed_receiver().len()
}

pub fn handed_callback(items: &[u8; 4], callback: fn(&u8)) {
    items.iter().for_each(callback);
}
