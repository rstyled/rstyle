#[cfg(test)]
mod rstyle_tagged_units_oracle {
    use super::{ByteClasses, Unit, UnitKind};

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
    #[test]
    fn classifiers_preserve_every_payload_and_distinguish_equal_numeric_tags() {
        for byte in 0..=u8::MAX {
            let unit = Unit::u8(byte);
            assert_eq!(unit.as_usize(), byte as usize);
            assert!(!unit.is_eoi());
        }
        for sentinel in 0..=u16::MAX {
            let unit = Unit(UnitKind::EOI(sentinel));
            assert_eq!(unit.as_usize(), sentinel as usize);
            assert!(unit.is_eoi());
        }
    }

    #[test]
    fn byte_class_lookup_preserves_mapping_and_full_sentinel_payloads() {
        for mapping in [[0u8; 256], core::array::from_fn(|index| 255 - index as u8)] {
            let classes = ByteClasses(mapping);
            for byte in 0..=u8::MAX {
                assert_eq!(
                    classes.get_by_unit(Unit::u8(byte)),
                    mapping[byte as usize] as usize
                );
            }
            for sentinel in 0..=u16::MAX {
                assert_eq!(
                    classes.get_by_unit(Unit(UnitKind::EOI(sentinel))),
                    sentinel as usize
                );
            }
        }
    }
}
