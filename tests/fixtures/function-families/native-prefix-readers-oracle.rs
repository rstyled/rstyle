fn expected_native(bytes: &[u8]) -> u128 {
    bytes.iter().enumerate().fold(0, |value, (index, &byte)| {
        let position = if cfg!(target_endian = "little") { index } else { bytes.len() - 1 - index };
        value | ((byte as u128) << (8 * position))
    })
}

#[test]
fn every_two_byte_word_preserves_native_position_and_ignores_the_tail() {
    for word in 0..=65535u32 {
        let bytes = [word as u8, (word >> 8) as u8, 0x55, 0xaa];
        let expected = expected_native(&bytes[..2]) as u16;
        assert_eq!(native_prefix_natural::read_u16(&bytes), expected);
        assert_eq!(read_u16(&bytes), expected);
        assert_eq!(native_u16([bytes[0], bytes[1]]), expected);
        assert_eq!(caller_u16(&bytes), expected);
    }
}

#[test]
fn wide_readers_preserve_each_bit_position_and_prefix_boundaries() {
    for bit in 0..128 {
        let mut bytes = [0u8; 20];
        bytes[bit / 8] = 1 << (bit % 8);
        bytes[16..].fill(0xff);
        let expected = expected_native(&bytes[..16]);
        assert_eq!(native_prefix_natural::read_u128(&bytes), expected);
        assert_eq!(read_u128(&bytes), expected);
        let expected32 = expected_native(&bytes[..4]) as u32;
        assert_eq!(native_prefix_natural::read_u32(&bytes), expected32);
        assert_eq!(read_u32(&bytes), expected32);
    }
    for byte in [0, 1, 0x80, 0xa5, 0xff] {
        let bytes = [byte; 20];
        assert_eq!(read_u128(&bytes), expected_native(&bytes[..16]));
        assert_eq!(native_prefix_natural::read_u128(&bytes), expected_native(&bytes[..16]));
        assert_eq!(read_u32(&bytes), expected_native(&bytes[..4]) as u32);
        assert_eq!(native_prefix_natural::read_u32(&bytes), expected_native(&bytes[..4]) as u32);
    }
}

#[test]
fn short_prefixes_panic_in_both_original_and_development_readers() {
    let bytes = [0xa5; 16];
    for length in 0..16 {
        let input = &bytes[..length];
        assert!(std::panic::catch_unwind(|| read_u128(input)).is_err());
        assert!(std::panic::catch_unwind(|| native_prefix_natural::read_u128(input)).is_err());
        if length < 4 {
            assert!(std::panic::catch_unwind(|| read_u32(input)).is_err());
            assert!(std::panic::catch_unwind(|| native_prefix_natural::read_u32(input)).is_err());
        }
        if length < 2 {
            assert!(std::panic::catch_unwind(|| read_u16(input)).is_err());
            assert!(std::panic::catch_unwind(|| native_prefix_natural::read_u16(input)).is_err());
        }
    }
}

#[test]
fn helper_selected_and_saved_prefixes_retain_the_evaluated_base() {
    let a = [0x12, 0x34, 0x99];
    let b = [0x56, 0x78, 0xff];
    assert_eq!(choose(&a, &b, true), &a);
    assert_eq!(choose(&a, &b, false), &b);
    assert_eq!(selected_u16(&a, &b, true), expected_native(&a[..2]) as u16);
    assert_eq!(selected_u16(&a, &b, false), expected_native(&b[..2]) as u16);
    assert_eq!(saved_u16(&a, &[]), expected_native(&a[..2]) as u16);
    assert_eq!(snapshot_u16(&a, &[]), expected_native(&a[..2]) as u16);
    assert_eq!(suffix(&a, 1), &a[1..]);
    assert_eq!(signed([0xff, 0xff]), -1);
}
