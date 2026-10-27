type AccelTy = u32;
const ACCEL_CAP: usize = 8;

#[derive(Clone)]
pub struct Accel {
    bytes: [u8; ACCEL_CAP],
}

impl Accel {
    pub fn new() -> Accel {
        Accel {
            bytes: [0; ACCEL_CAP],
        }
    }
    pub fn len(&self) -> usize {
        usize::from(self.bytes[0])
    }
    pub fn as_accel_tys(&self) -> [AccelTy; 2] {
        assert_eq!(ACCEL_CAP, 8);
        let first = AccelTy::from_ne_bytes(self.bytes[0..4].try_into().unwrap());
        let second = AccelTy::from_ne_bytes(self.bytes[4..8].try_into().unwrap());
        [first, second]
    }
}

pub fn getter(value: &Accel) -> [AccelTy; 2] {
    value.as_accel_tys()
}
pub fn first(value: &Accel) -> AccelTy {
    value.as_accel_tys()[0]
}
pub fn saved(a: &Accel, b: &Accel) -> [AccelTy; 2] {
    let mut selected = a;
    let words = selected.as_accel_tys();
    selected = b;
    let _ = selected;
    words
}
pub fn selected(a: &Accel, b: &Accel, pick: bool) -> [AccelTy; 2] {
    let selected = if pick { a } else { b };
    selected.as_accel_tys()
}
pub fn scalar_alias(value: AccelTy) -> AccelTy {
    value
}
pub fn array_alias(bytes: [u8; 4]) -> [AccelTy; 2] {
    [AccelTy::from_ne_bytes(bytes), 0]
}
pub fn compare_order(mut n: u32) -> u32 {
    assert_eq!(
        {
            n += 1;
            n
        },
        {
            n += 1;
            n - 1
        }
    );
    n
}
pub fn debug_order(mut n: u32) -> u32 {
    debug_assert_eq!(
        {
            n += 1;
            n
        },
        {
            n += 1;
            n - 1
        }
    );
    n
}
pub fn owned(value: Accel) -> [AccelTy; 2] {
    value.as_accel_tys()
}
pub fn mutable(value: &mut Accel) -> [AccelTy; 2] {
    value.as_accel_tys()
}
pub fn formatted(n: usize) {
    assert_eq!(n, 8, "bad {n}");
}
pub fn saved_result(value: &Accel) -> [u8; 4] {
    let result: Result<[u8; 4], _> = value.bytes[0..4].try_into();
    result.unwrap()
}
pub fn raw(bytes: &[u8]) -> u32 {
    u32::from_ne_bytes(bytes[0..4].try_into().unwrap())
}
pub fn raw_caller(bytes: &[u8]) -> u32 {
    raw(bytes)
}
pub mod counterfeit {
    macro_rules! assert_eq {
        ($left:expr, $right:expr) => {
            ()
        };
    }
    pub fn compare(n: usize) {
        assert_eq!(n, 8);
    }
}
