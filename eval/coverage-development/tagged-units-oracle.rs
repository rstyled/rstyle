#[cfg(test)]
mod rstyle_tagged_units_oracle {
    use super::{Unit, UnitKind};

    #[test]
    fn every_byte_roundtrips_and_has_no_sentinel_payload() {
        for byte in 0..=u8::MAX {
            let unit = Unit::u8(byte);
            assert_eq!(unit.as_u8(), Some(byte));
            assert_eq!(unit.as_eoi(), None);
            assert!(!unit.is_eoi());
            assert_eq!(unit.as_u8(), Some(byte));
        }
    }

    #[test]
    fn every_sentinel_payload_is_preserved_without_narrowing() {
        for sentinel in 0..=u16::MAX {
            let unit = Unit(UnitKind::EOI(sentinel));
            assert_eq!(unit.as_u8(), None);
            assert_eq!(unit.as_eoi(), Some(sentinel));
            assert!(unit.is_eoi());
            assert_eq!(unit.as_eoi(), Some(sentinel));
        }
    }

    #[test]
    fn public_sentinel_constructor_keeps_its_checked_domain() {
        for sentinel in 0..=256usize {
            let unit = Unit::eoi(sentinel);
            assert_eq!(unit.as_u8(), None);
            assert_eq!(unit.as_eoi(), Some(sentinel as u16));
        }
        for invalid in [257, 65536, usize::MAX] {
            assert!(std::panic::catch_unwind(|| Unit::eoi(invalid)).is_err());
        }
    }
}
