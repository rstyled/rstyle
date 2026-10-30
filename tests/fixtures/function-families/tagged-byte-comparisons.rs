// Append to tagged-units.rs; preserve its original eleven-function corpus.
impl Unit {
    pub fn is_byte(self, expected: u8) -> bool {
        self.byte().map_or(false, |actual| actual == expected)
    }
}

pub fn byte_matches(actual: u8, expected: u8) -> bool {
    Unit::new_byte(actual).is_byte(expected)
}

pub fn end_never_matches(end: u16, expected: u8) -> bool {
    Unit::new_end(end).is_byte(expected)
}

pub fn saved_comparison(actual: u8, mut expected: u8) -> bool {
    let saved = Unit::new_byte(actual).is_byte(expected);
    expected = 0;
    let _ = expected;
    saved
}

pub fn local_option(actual: u8, expected: u8) -> bool {
    let value = Some(actual);
    value.map_or(false, |byte| byte == expected)
}

pub fn default_before_capture(actual: u8, mut expected: u8) -> bool {
    Some(actual).map_or(
        {
            expected = 1;
            false
        },
        |byte| byte == expected,
    )
}

pub fn none_uses_default(expected: u8) -> bool {
    let value: Option<u8> = None;
    value.map_or(true, |byte| byte == expected)
}

pub fn callback_reference_payload(value: Option<&u8>, expected: u8) -> bool {
    value.map_or(false, |byte| *byte == expected)
}

pub fn mutating_capture(value: Option<u8>, counter: &mut u8) -> bool {
    value.map_or(false, |byte| {
        *counter += byte;
        true
    })
}

pub fn escaping(actual: u8, expected: u8) -> impl FnOnce() -> bool {
    move || actual == expected
}

pub fn nested_callback(value: Option<u8>, expected: u8) -> bool {
    value.map_or(false, |byte| {
        Some(byte).map_or(false, |inner| inner == expected)
    })
}

pub mod callback_counterfeit {
    #[derive(Clone, Copy)]
    pub struct Other(pub u8);

    impl Other {
        pub fn map_or(self, default: bool, callback: fn(u8) -> bool) -> bool {
            std::hint::black_box(callback(self.0) || default)
        }
    }
}

pub fn custom_map(value: callback_counterfeit::Other) -> bool {
    value.map_or(false, |byte| byte == 0)
}

pub fn callback_panics(value: Option<u8>) -> bool {
    value.map_or(false, |_| panic!("callback effect"))
}
