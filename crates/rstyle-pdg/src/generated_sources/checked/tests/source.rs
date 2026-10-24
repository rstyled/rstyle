mod util {
    pub mod primitives {
        macro_rules! index_type_impls {
            ($name:ident, $err:ident, $iter:ident, $withiter:ident) => {
                impl $name {
                    pub const MAX: $name = $name(SmallIndex::MAX);
                    pub const LIMIT: usize = SmallIndex::LIMIT;
                    pub const ZERO: $name = $name(SmallIndex::ZERO);
                    pub const SIZE: usize = SmallIndex::SIZE;
                    #[inline]
                    pub fn new(value: usize) -> Result<$name, $err> {
                        SmallIndex::new(value).map($name).map_err($err)
                    }
                    pub fn new_unchecked() -> usize { 0 }
                    pub fn must() -> usize { 0 }
                    pub fn as_usize() -> usize { 0 }
                    pub fn as_u64() -> usize { 0 }
                    pub fn as_u32() -> usize { 0 }
                    pub fn as_i32() -> usize { 0 }
                    pub fn one_more() -> usize { 0 }
                    pub fn from_ne_bytes() -> usize { 0 }
                    pub fn from_ne_bytes_unchecked() -> usize { 0 }
                    pub fn to_ne_bytes() -> usize { 0 }
                    pub fn iter() -> usize { 0 }
                }
                pub struct $err(SmallIndexError);
            };
        }
        index_type_impls!(PatternID, PatternIDError, PatternIDIter, WithPatternIDIter);
    }
}
