#[cfg(test)]
mod rstyle_byte_scalar_classifiers_oracle {
    #[test]
    fn original_classifier_matches_the_independent_ascii_bitset_for_every_byte() {
        let alphabet = [0x03ff000000000000u64, 0x07fffffe87fffffe, 0, 0];
        for byte in 0..=u8::MAX {
            let expected = alphabet[usize::from(byte) / 64] & (1u64 << (byte % 64)) != 0;
            assert_eq!(super::is_word_byte(byte), expected, "byte={byte}");
        }
    }
}
