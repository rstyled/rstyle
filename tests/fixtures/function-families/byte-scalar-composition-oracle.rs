#[cfg(test)]
mod rstyle_byte_scalar_composition_oracle {
    #[test]
    fn saved_checked_and_byte_scalar_compositions_match_independent_expectations() {
        let alphabet = [0x03ff000000000000u64, 0x07fffffe87fffffe, 0, 0];
        for byte in 0..=u8::MAX {
            let expected = alphabet[usize::from(byte) / 64] & (1u64 << (byte % 64)) != 0;
            assert_eq!(super::caller(byte), expected);
            assert_eq!(super::saved(byte), expected);
            assert_eq!(super::checked(&[byte], 0), expected);
            assert!(!super::checked(&[byte], 1));
            assert!(!super::checked(&[], usize::from(byte)));
            assert_eq!(super::underscore(byte), byte == 95);
            assert_eq!(super::exhaustive(byte), byte >= 128);
            assert_eq!(super::mixed(byte), byte <= 57);
        }
        assert_eq!(super::escaped(), 10);
        assert_eq!(super::as_index(), 95);
    }
}
