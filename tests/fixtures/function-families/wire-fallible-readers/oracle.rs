// Appended to the complete original util::wire module. No production body copied.
#[doc(hidden)]
pub(crate) fn __rstyle_wire_fallible_readers_oracle() -> [u64; 4] {
    fn buffer_payload(error: DeserializeError) -> &'static str {
        match error.0 {
            DeserializeErrorKind::BufferTooSmall { what } => what,
            _ => panic!("wrong original error variant or short-input precedence"),
        }
    }
    fn buffer_error(error: DeserializeError, expected: &'static str) {
        let what = buffer_payload(error);
        assert_eq!(what.len(), expected.len());
        assert_eq!(what.as_ptr(), expected.as_ptr());
    }
    fn decoded(bytes: &[u8]) -> u128 {
        let mut value = 0;
        // Arithmetic expectation, independent of from_ne_bytes/try_into/read helpers.
        if cfg!(target_endian = "little") {
            for byte in bytes.iter().rev() {
                value = value * 256 + u128::from(*byte);
            }
        } else {
            for byte in bytes {
                value = value * 256 + u128::from(*byte);
            }
        }
        value
    }
    fn length(slice: &[u8], bound: usize, what: &'static str) {
        match check_slice_len::<u8>(slice, bound, what) {
            Ok(()) => assert!(slice.len() >= bound),
            Err(error) => {
                assert!(slice.len() < bound);
                buffer_error(error, what);
            }
        }
    }
    fn reader(slice: &[u8], width: usize, what: &'static str) {
        let actual = match width {
            2 => try_read_u16(slice, what).map(|(n, nr)| (u128::from(n), nr)),
            4 => try_read_u32(slice, what).map(|(n, nr)| (u128::from(n), nr)),
            16 => try_read_u128(slice, what),
            _ => panic!("oracle width outside frozen family"),
        };
        if slice.len() < width {
            buffer_error(actual.unwrap_err(), what);
        } else {
            let expected = decoded(&slice[..width]);
            assert_eq!(actual.unwrap(), (expected, width));
            let direct = match width {
                2 => u128::from(read_u16(slice)),
                4 => u128::from(read_u32(slice)),
                16 => read_u128(slice),
                _ => unreachable!(),
            };
            assert_eq!(direct, expected);
        }
    }
    fn version(slice: &[u8], expected: u32) {
        let actual = read_version(slice, expected);
        if slice.len() < 4 {
            assert_eq!(buffer_payload(actual.unwrap_err()), "version");
            return;
        }
        let found = decoded(&slice[..4]) as u32;
        if found == expected {
            assert_eq!(actual.unwrap(), 4);
        } else {
            match actual.unwrap_err().0 {
                DeserializeErrorKind::VersionMismatch {
                    expected: got,
                    found: got_found,
                } => {
                    assert_eq!((got, got_found), (expected, found));
                }
                _ => panic!("wrong original version mismatch fields"),
            }
        }
    }
    fn endian(slice: &[u8]) {
        let actual = read_endianness_check(slice);
        if slice.len() < 4 {
            assert_eq!(buffer_payload(actual.unwrap_err()), "endianness check");
            return;
        }
        let found = decoded(&slice[..4]) as u32;
        if found == 0xFEFF {
            assert_eq!(actual.unwrap(), 4);
        } else {
            match actual.unwrap_err().0 {
                DeserializeErrorKind::EndianMismatch {
                    expected,
                    found: got,
                } => {
                    assert_eq!((expected, got), (0xFEFF, found));
                }
                _ => panic!("wrong original endian mismatch fields"),
            }
        }
    }
    assert_eq!(usize::BITS, 64);
    let messages = ["", "wire field A", "wire field B"];
    let mut counts = [0u64; 4];
    for what in messages {
        buffer_error(DeserializeError::buffer_too_small(what), what);
        counts[0] += 1;
    }
    let bytes = [0xA5; 64];
    for offset in 0..4 {
        for len in 0..=32 {
            for bound in (0..=40).chain([63, 64, 65, usize::MAX]) {
                for what in messages {
                    length(&bytes[offset..offset + len], bound, what);
                    counts[1] += 1;
                }
            }
        }
    }
    for seed in 0u16..=255 {
        let mut bytes = [0; 64];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = (seed as u8).wrapping_add((i as u8).wrapping_mul(37));
        }
        for offset in 0..4 {
            for len in 0..=24 {
                let slice = &bytes[offset..offset + len];
                for width in [2, 4, 16] {
                    for what in messages {
                        reader(slice, width, what);
                        counts[2] += 1;
                    }
                }
                if len <= 8 {
                    endian(slice);
                    counts[3] += 1;
                    let expected = decoded(&bytes[offset..offset + 4]) as u32;
                    for version_number in [0, 0xFEFF, expected] {
                        version(slice, version_number);
                        counts[3] += 1;
                    }
                }
            }
        }
    }
    // Explicit endian successes and scalar boundaries are independent of the
    // patterned corpus, which need not contain the 0xFEFF native representation.
    for number in [0u32, 1, 0xFEFF, 0xFFFE, 0xFFFF, 0x10000, u32::MAX] {
        let mut bytes = [0xA5; 8];
        let mut remaining = number;
        for i in 0..4 {
            let slot = if cfg!(target_endian = "little") {
                i
            } else {
                3 - i
            };
            bytes[slot] = (remaining % 256) as u8;
            remaining /= 256;
        }
        for len in 0..=8 {
            let slice = &bytes[..len];
            endian(slice);
            counts[3] += 1;
            for expected in [0, 0xFEFF, number] {
                version(slice, expected);
                counts[3] += 1;
            }
        }
    }
    counts
}
