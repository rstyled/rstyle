// Append to option-byte-widths.rs in an independent rustc --test harness.
#[cfg(test)]
mod oracle {
    use super::*;

    #[test]
    fn all_bytes_follow_the_explicit_width_partition() {
        let partitions = [
            (0u8, 127u8, Some(1usize)),
            (128, 191, None),
            (192, 223, Some(2)),
            (224, 239, Some(3)),
            (240, 247, Some(4)),
            (248, 255, None),
        ];
        for (low, high, expected) in partitions {
            for byte in low..=high {
                assert_eq!(match_width(byte), expected, "match {byte}");
                assert_eq!(branch_width(byte), expected, "branch {byte}");
                for fallback in [0, 5, usize::MAX] {
                    assert_eq!(width_or(byte, fallback), expected.unwrap_or(fallback));
                }
                assert_eq!(saved_width(byte), expected.unwrap_or(0));
            }
        }
    }

    #[test]
    fn width_is_not_unicode_scalar_validity() {
        for byte in [0xC0, 0xC1] { assert_eq!(match_width(byte), Some(2)); }
        for byte in [0xF5, 0xF6, 0xF7] { assert_eq!(branch_width(byte), Some(4)); }
    }
}
