#[derive(Clone, Copy)]
pub struct Unit(UnitKind);
#[derive(Clone, Copy)]
enum UnitKind {
    U8(u8),
    EOI(u16),
}
impl Unit {
    pub fn eoi(n: usize) -> Unit {
        assert!(n <= 256, "max number of classes is 256, but got {n}");
        Unit(UnitKind::EOI(u16::try_from(n).unwrap()))
    }
    pub fn as_usize(self) -> usize {
        match self.0 {
            UnitKind::U8(byte) => usize::from(byte),
            UnitKind::EOI(sentinel) => usize::from(sentinel),
        }
    }
}
#[derive(Clone, Copy)]
pub struct ByteClasses([u8; 256]);
impl ByteClasses {
    pub fn empty() -> ByteClasses {
        ByteClasses([0; 256])
    }
    pub fn alphabet_len(&self) -> usize {
        usize::from(self.0[255]) + 1 + 1
    }
    pub fn eoi(&self) -> Unit {
        Unit::eoi(self.alphabet_len().checked_sub(1).unwrap())
    }
}
pub fn direct(n: usize) -> usize {
    Unit::eoi(n).as_usize()
}
pub fn saved(n: usize) -> usize {
    let unit = Unit::eoi(n);
    unit.as_usize()
}
pub fn mapped(classes: &ByteClasses) -> usize {
    classes.eoi().as_usize()
}
pub fn checked(n: usize, amount: usize) -> usize {
    n.checked_sub(amount).unwrap()
}
pub fn saved_result(n: usize) -> u16 {
    let result = u16::try_from(n);
    result.unwrap()
}
