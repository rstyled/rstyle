// Source-selected checksum-state roundtrips, declared before admission assessment.
pub mod state {
    #[derive(Clone, Copy, Debug)]
    pub struct State { low: u16, high: u16 }

    impl State {
        pub const fn from_checksum(sum: u32) -> Self {
            Self { low: sum as u16, high: (sum >> 16) as u16 }
        }
        pub fn checksum(&self) -> u32 {
            (u32::from(self.high) << 16) | u32::from(self.low)
        }
    }
}

pub fn roundtrip(sum: u32) -> u32 { state::State::from_checksum(sum).checksum() }
pub fn saved_input(mut sum: u32) -> u32 {
    let saved = state::State::from_checksum(sum);
    sum = 0;
    saved.checksum() | sum
}
pub fn widen(value: u16) -> u32 { u32::from(value) }

mod custom {
    #[derive(Clone, Copy)]
    pub struct Custom { pub value: u32 }
    impl From<u16> for Custom {
        fn from(value: u16) -> Self {
            if value == 0 { panic!("custom conversion"); }
            Self { value: value as u32 }
        }
    }
    pub fn custom_conversion(value: u16) -> u32 { Custom::from(value).value }
}
pub fn floating(value: u32) -> f64 { f64::from(value) }
