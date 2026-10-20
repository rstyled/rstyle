#[cfg(test)]
mod rstyle_checked_native_oracle {
    use super::*;

    fn expected(bytes: &[u8]) -> u128 {
        bytes.iter().enumerate().fold(0, |value, (index, &byte)| {
            let position = if cfg!(target_endian = "little") {
                index
            } else {
                bytes.len() - index - 1
            };
            value | ((byte as u128) << (position * 8))
        })
    }

    fn check<T: core::fmt::Debug + PartialEq>(
        result: Result<(T, usize), DeserializeError>,
        input: &[u8],
        width: usize,
        value: T,
        description: &'static str,
    ) {
        if input.len() >= width {
            assert_eq!(result.unwrap(), (value, width));
        } else {
            match result.unwrap_err().0 {
                DeserializeErrorKind::BufferTooSmall { what } => {
                    assert_eq!(what, description);
                    assert_eq!(what.as_ptr(), description.as_ptr());
                }
                error => panic!("wrong error variant: {error:?}"),
            }
        }
    }

    fn check_all(input: &[u8], description: &'static str) {
        let value = |width| {
            if input.len() >= width {
                expected(&input[..width])
            } else {
                0
            }
        };
        check(
            try_read_u16(input, description),
            input,
            2,
            value(2) as u16,
            description,
        );
        check(
            try_read_u32(input, description),
            input,
            4,
            value(4) as u32,
            description,
        );
        check(
            try_read_u128(input, description),
            input,
            16,
            value(16),
            description,
        );
    }

    #[test]
    fn every_short_length_and_tail_preserves_result_and_description() {
        for description in [
            "",
            "reader",
            "a longer static description with spaces and punctuation!",
        ] {
            for byte in [0, 1, 0x80, 0xa5, 0xff] {
                let bytes = [byte; 24];
                for length in 0..=24 {
                    check_all(&bytes[..length], description);
                }
            }
        }
    }

    #[test]
    fn each_wide_bit_has_independently_expected_native_significance() {
        for bit in 0..128 {
            let mut bytes = [0; 24];
            bytes[bit / 8] = 1 << (bit % 8);
            bytes[16..].fill(0xff);
            check_all(&bytes, "wide bit");
        }
    }

    #[test]
    fn every_two_byte_word_ignores_trailing_bytes() {
        for word in 0..=65535u32 {
            let bytes = [word as u8, (word >> 8) as u8, 0xa5, 0xff];
            let value = expected(&bytes[..2]) as u16;
            assert_eq!(try_read_u16(&bytes, "word").unwrap(), (value, 2));
            assert_eq!(try_read_u16(&bytes[..2], "word").unwrap(), (value, 2));
        }
    }
}
