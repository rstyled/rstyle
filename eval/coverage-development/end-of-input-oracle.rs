#[cfg(test)]
mod rstyle_end_of_input_oracle {
    use super::{ByteClasses, Unit, UnitKind};
    use std::{format, string::String};

    #[test]
    fn constructor_preserves_domain_and_diagnostic() {
        for value in 0..=256usize {
            let Unit(UnitKind::EOI(payload)) = Unit::eoi(value) else {
                panic!("wrong tag");
            };
            assert_eq!(payload as usize, value);
        }
        for value in [257, 65535, 65536, usize::MAX] {
            let panic = std::panic::catch_unwind(|| Unit::eoi(value)).unwrap_err();
            let message = panic.downcast_ref::<String>().map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied()).unwrap();
            assert_eq!(message, format!(
                "max number of byte-based equivalent classes is 256, but got {value}"
            ));
        }
    }

    #[test]
    fn sentinel_uses_last_byte_for_every_value_and_varied_neighbors() {
        for last in 0..=255u16 {
            for fill in [0, 1, 127, 255] {
                let mut map = [fill; 256];
                map[255] = last as u8;
                let Unit(UnitKind::EOI(payload)) = ByteClasses(map).eoi() else {
                    panic!("wrong tag");
                };
                assert_eq!(payload, last + 1);
            }
            let mut map = core::array::from_fn(|index| index as u8);
            map[255] = last as u8;
            let Unit(UnitKind::EOI(payload)) = ByteClasses(map).eoi() else {
                panic!("wrong tag");
            };
            assert_eq!(payload, last + 1);
        }
    }
}
