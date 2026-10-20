// Separate development selection: read-only borrowed owners are not Copy.
pub struct ScalarRange { pub start: u32, pub end: u32 }
impl ScalarRange {
    pub fn is_valid(&self) -> bool { self.start <= self.end }
    pub fn is_ascii(&self) -> bool { self.is_valid() && self.end <= 127 }
    pub fn consume(self) -> u32 { self.start }
    pub fn owned(start: u32, end: u32) -> Self { Self { start, end } }
}
pub fn borrowed(range: &ScalarRange) -> bool { range.is_ascii() }
pub fn same(range: &ScalarRange) -> bool { range.is_valid() && range.end <= 127 }
pub fn local_owned(start: u32, end: u32) -> bool {
    let range = ScalarRange { start, end }; range.is_ascii()
}
pub enum Neighbor { Empty, Number(u32) }
pub struct Dropped { pub start: u32, pub end: u32 }
impl Drop for Dropped { fn drop(&mut self) {} }
impl Dropped {
    pub fn borrowed(&self) -> bool { self.start <= self.end }
    pub fn consume(self) -> bool { self.start <= self.end }
}
