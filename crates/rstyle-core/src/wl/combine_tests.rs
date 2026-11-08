use super::combine;

/// Reference FNV-1a over both operands, obtained by shifts rather than byte arrays.
/// The first operand is hashed input, never the initial hash state.
fn shifted_byte_oracle(first: u64, second: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for position in 0..16 {
        let operand = if position < 8 { first } else { second };
        let byte = (operand >> ((position % 8) * 8)) & 0xff;
        hash = (hash ^ byte).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[test]
fn combine_hashes_both_complete_operands_from_the_frozen_offset() {
    let cases = [
        (0, 0),
        (0, u64::MAX),
        (u64::MAX, 0),
        (u64::MAX, u64::MAX),
        (0xcbf2_9ce4_8422_2325, 0x0000_0100_0000_01b3),
        (0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210),
    ];
    for (first, second) in cases {
        assert_eq!(combine(first, second), shifted_byte_oracle(first, second));
        assert_eq!(combine(second, first), shifted_byte_oracle(second, first));
    }
    assert_ne!(
        combine(0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210),
        combine(0xfedc_ba98_7654_3210, 0x0123_4567_89ab_cdef)
    );
}

#[test]
fn combine_preserves_every_bit_in_each_operand() {
    for bit in 0..64 {
        for other in [0, u64::MAX, 0x0123_4567_89ab_cdef] {
            let selected = 1u64 << bit;
            assert_eq!(
                combine(selected, other),
                shifted_byte_oracle(selected, other)
            );
            assert_eq!(
                combine(other, selected),
                shifted_byte_oracle(other, selected)
            );
        }
    }
}

#[test]
fn combine_matches_independent_oracle_for_every_byte_pair() {
    for first in 0..=u64::from(u8::MAX) {
        for second in 0..=u64::from(u8::MAX) {
            assert_eq!(combine(first, second), shifted_byte_oracle(first, second));
        }
    }
}
