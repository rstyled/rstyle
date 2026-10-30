// Independent expectations: ordinal bit positions and masks, not implementation calls.
#[test]
fn all_flag_sets_obey_the_documented_membership_algebra() {
    let flags = [Look::Start, Look::End, Look::StartLF, Look::EndLF,
        Look::StartCRLF, Look::EndCRLF, Look::WordAscii, Look::WordAsciiNegate,
        Look::WordUnicode, Look::WordUnicodeNegate, Look::WordStartAscii,
        Look::WordEndAscii, Look::WordStartUnicode, Look::WordEndUnicode,
        Look::WordStartHalfAscii, Look::WordEndHalfAscii,
        Look::WordStartHalfUnicode, Look::WordEndHalfUnicode];
    for bits in 0..(1 << 18) {
        let set = LookSet { bits };
        assert_eq!(set.contains_anchor(), bits & 0x3f != 0);
        assert_eq!(set.contains_anchor_haystack(), bits & 0x03 != 0);
        assert_eq!(set.contains_anchor_line(), bits & 0x3c != 0);
        assert_eq!(set.contains_anchor_lf(), bits & 0x0c != 0);
        assert_eq!(set.contains_anchor_crlf(), bits & 0x30 != 0);
        assert_eq!(set.contains_word(), bits & 0x3ffc0 != 0);
        assert_eq!(set.contains_word_ascii(), bits & 0x0ccc0 != 0);
        assert_eq!(set.contains_word_unicode(), bits & 0x33300 != 0);
        for (position, flag) in flags.iter().copied().enumerate() {
            let mask = 1u32 << position;
            assert_eq!(flag.as_repr(), mask);
            assert_eq!(set.contains(flag), bits & mask != 0);
            assert_eq!(LookSet::singleton(flag).bits, mask);
            assert_eq!(set.insert(flag).bits, bits | mask);
            assert_eq!(set.insert(flag).insert(flag).bits, bits | mask);
            assert_eq!(set.remove(flag).bits, bits & !mask);
            assert_eq!(set.remove(flag).remove(flag).bits, bits & !mask);
            assert_eq!(set.bits, bits);
        }
    }
    let unrelated = LookSet { bits: 0xfffc0000 };
    assert!(!unrelated.contains_anchor());
    assert!(!unrelated.contains_word());
    assert_eq!(unrelated.insert(Look::Start).bits, 0xfffc0001);
    assert_eq!(unrelated.remove(Look::Start).bits, 0xfffc0000);
}
