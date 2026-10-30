#[test]
fn checksum_roundtrips_preserve_the_entire_input() {
    for high in 0u32..=65535 {
        for low in [0, 1, 32767, 65535] {
            let sum = high * 65536 + low;
            let natural = adler2::Adler32::from_checksum(sum).checksum();
            assert_eq!(natural, sum);
            assert_eq!(state::State::from_checksum(sum).checksum(), sum);
            assert_eq!(roundtrip(sum), sum);
            assert_eq!(saved_input(sum), sum);
        }
    }
}

#[test]
fn widening_preserves_every_u16_value() {
    assert_eq!(custom::custom_conversion(1), 1);
    assert!(std::panic::catch_unwind(|| custom::custom_conversion(0)).is_err());
    assert_eq!(floating(1), 1.0);
    for value in 0u16..=65535 {
        assert_eq!(widen(value), value as u32);
    }
}
