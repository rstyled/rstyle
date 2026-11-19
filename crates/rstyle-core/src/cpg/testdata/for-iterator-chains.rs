pub fn enumerate(items: &[u8]) {
    for (_index, _item) in items.iter().enumerate() {}
}

pub fn reverse(items: &[u8]) {
    for _item in items.iter().rev() {}
}

pub fn fixed(items: &[u8; 4]) {
    for _item in items.iter().rev() {}
}

pub fn callback_cost(items: &[u8; 4]) {
    for _item in items.iter().filter(|_| {
        for _index in 0..3usize {}
        true
    }) {
        for _index in 0..2usize {}
    }
}

pub fn early_break(items: &[u8; 4]) {
    for item in items.iter().rev() {
        if *item == 0 {
            break;
        }
        for _index in 0..2usize {}
    }
}

pub fn early_continue(items: &[u8; 4]) {
    for item in items.iter().rev() {
        if *item == 0 {
            continue;
        }
        for _index in 0..2usize {}
    }
}

pub fn early_return(items: &[u8; 4]) {
    for item in items.iter().rev() {
        if *item == 0 {
            return;
        }
        for _index in 0..2usize {}
    }
}

pub fn callback_return(items: &[u8; 4]) {
    for _item in items.iter().map(|item| {
        if *item == 0 {
            return 0;
        }
        for _index in 0..3usize {}
        1
    }) {
        for _index in 0..2usize {}
    }
}

pub fn argument() -> usize {
    for _index in 0..3usize {}
    1
}

pub fn evaluated_once(items: &[u8]) {
    for _item in items.iter().rev().skip(argument()).take(2) {}
}

pub fn mutable_source(items: &mut [u8]) {
    for _item in items.iter_mut().rev() {}
}

pub fn unknown_callback(items: &[u8], predicate: fn(&&u8) -> bool) {
    for _item in items.iter().filter(predicate) {}
}

pub fn unknown_source(items: impl DoubleEndedIterator<Item = u8>) {
    for _item in items.rev() {}
}

pub fn unmodeled_flat_map(items: &[u8]) {
    for _item in items.iter().flat_map(|_| 0..3usize) {}
}

pub fn growing_source(items: &std::cell::RefCell<Vec<u8>>) {
    for _item in std::iter::from_fn(|| {
        items.borrow_mut().push(0);
        Some(0u8)
    }).enumerate() {}
}

pub struct Hook;

impl Clone for Hook {
    fn clone(&self) -> Self {
        loop {}
    }
}

pub fn custom_clone(items: &[Hook]) {
    for _item in items.iter().cloned() {}
}

pub fn callback_unconditional_return(items: &[u8; 4]) {
    for _item in items.iter().map(|_| { return 0u8; }) {
        for _index in 0..2usize {}
    }
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        loop {}
    }
}

pub struct Wrapped {
    pub guard: Guard,
}

pub fn make_guard(_item: &u8) -> Guard {
    Guard
}

pub fn make_wrapped(_item: &u8) -> Wrapped {
    Wrapped { guard: Guard }
}

pub fn owned_drop(items: &[u8]) {
    for _guard in items.iter().map(make_guard) {}
}

pub fn nested_drop(items: &[u8]) {
    for _wrapped in items.iter().map(make_wrapped) {}
}

pub fn tuple_drop(items: &[u8]) {
    for (_index, _guard) in items.iter().map(make_guard).enumerate() {}
}

pub fn fixed_yield(items: &[u8; 4]) {
    for _array in items.iter().map(|_| [0u8; 2]) {}
}

pub fn ref_drop(items: &[u8]) {
    for ref _guard in items.iter().map(make_guard) {}
}

pub fn consume_guard(_guard: Guard) -> u8 {
    0
}

pub fn intermediate_drop(items: &[u8]) {
    for _item in items.iter().map(make_guard).map(consume_guard) {}
}

pub fn guard_value(_guard: &Guard) -> u8 {
    0
}

pub fn captured_guard(items: &[u8], guard: Guard) {
    for _item in items.iter().map(move |_| guard_value(&guard)) {}
}

pub fn shared_capture(items: &[u8; 4], guard: &Guard) {
    for _item in items.iter().map(|_| guard_value(guard)) {}
}

pub fn moved_reference_capture(items: &[u8; 4], guard: &Guard) {
    for _item in items.iter().map(move |_| guard_value(guard)) {}
}

pub fn terminal(items: &[u8; 4]) -> usize {
    items.iter().map(|_| 1u8).count()
}

pub fn named_item(_item: &u8) -> u8 {
    for _index in 0..3usize {}
    1
}

pub fn named_map(items: &[u8; 4]) {
    for _item in items.iter().map(named_item) {}
}
