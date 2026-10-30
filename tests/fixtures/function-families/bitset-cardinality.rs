#[derive(Clone, Copy)]
pub struct LookSet { pub bits: u32 }
impl LookSet {
    pub fn empty() -> Self { Self { bits: 0 } }
    pub fn full() -> Self { Self { bits: !0 } }
    pub fn len(self) -> usize { usize::try_from(self.bits.count_ones()).unwrap() }
    pub fn is_empty(self) -> bool { self.len() == 0 }
}
pub fn count(value: LookSet) -> usize { value.len() }
pub fn empty(value: LookSet) -> bool { value.is_empty() }
pub fn saved(value: LookSet) -> usize { let size = value.len(); size }
pub fn selected(a: LookSet, b: LookSet, pick: bool) -> usize {
    let value = if pick { a } else { b }; value.len()
}
pub fn mutate(value: &mut LookSet) -> usize { value.bits = 0; value.len() }
pub struct Custom;
impl Custom { pub fn count_ones(self) -> u32 { 99 } }
pub fn custom(value: Custom) -> usize { usize::try_from(value.count_ones()).unwrap() }
pub fn narrowing(value: u32) -> u8 { u8::try_from(value).unwrap() }
pub fn propagate(value: u32) -> Result<usize, core::num::TryFromIntError> {
    Ok(usize::try_from(value)?)
}
