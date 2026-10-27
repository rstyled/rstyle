#[cfg(test)]
mod rstyle_bitset_cardinality_oracle {
    use super::LookSet;

    fn expected(mut bits: u32) -> usize {
        let mut sum = 0;
        for _ in 0..32 {
            sum += (bits % 2) as usize;
            bits /= 2;
        }
        sum
    }

    fn check(bits: u32) {
        let set = LookSet { bits };
        assert_eq!(set.len(), expected(bits), "{bits:#010x}");
        assert_eq!(set.is_empty(), bits == 0, "{bits:#010x}");
    }

    #[test]
    fn cardinality_covers_every_byte_in_every_lane_and_extreme_bits() {
        for lane in 0..4 {
            for byte in 0..=255u32 {
                check(byte << (8 * lane));
                check(!(byte << (8 * lane)));
            }
        }
        for bit in 0..32 {
            check(1 << bit);
            check(!(1 << bit));
        }
        for bits in [0, u32::MAX, 0xaaaa_aaaa, 0x5555_5555, 0x8000_0001] {
            check(bits);
        }
    }

    #[test]
    fn cardinality_matches_independent_arithmetic_on_mixed_words() {
        let mut bits = 0x9e37_79b9u32;
        for _ in 0..4096 {
            bits = bits.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            check(bits);
        }
        assert_eq!(LookSet::empty().len(), 0);
        assert_eq!(LookSet::full().len(), 32);
    }
}
