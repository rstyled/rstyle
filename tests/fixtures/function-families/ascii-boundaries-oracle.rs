#[cfg(test)]
mod rstyle_ascii_boundaries_oracle {
    use super::LookMatcher;

    const ASCII_WORD: [u64; 4] = [0x03ff000000000000, 0x07fffffe87fffffe, 0, 0];

    fn word(byte: u8) -> bool {
        ASCII_WORD[usize::from(byte) / 64] & (1u64 << (byte % 64)) != 0
    }

    fn results(matcher: &LookMatcher, bytes: &[u8], at: usize) -> [bool; 6] {
        [
            matcher.is_word_ascii(bytes, at),
            matcher.is_word_ascii_negate(bytes, at),
            matcher.is_word_start_ascii(bytes, at),
            matcher.is_word_end_ascii(bytes, at),
            matcher.is_word_start_half_ascii(bytes, at),
            matcher.is_word_end_half_ascii(bytes, at),
        ]
    }

    fn check(matcher: &LookMatcher, bytes: &[u8]) {
        for at in 0..=bytes.len() {
            let before = at > 0 && word(bytes[at - 1]);
            let after = at < bytes.len() && word(bytes[at]);
            assert_eq!(results(matcher, bytes, at), [
                before != after, before == after, !before && after,
                before && !after, !before, !after,
            ], "{bytes:?}@{at}");
        }
    }

    #[test]
    fn all_bytes_pairs_and_empty_boundaries_follow_independent_membership() {
        let matcher = LookMatcher::new();
        check(&matcher, &[]);
        for first in 0..=u8::MAX {
            check(&matcher, &[first]);
            for second in 0..=u8::MAX {
                check(&matcher, &[first, second]);
            }
        }
    }

    #[test]
    fn line_terminators_do_not_change_ascii_boundaries() {
        for terminator in [0, 10, 95, 255] {
            let mut matcher = LookMatcher::new();
            matcher.set_line_terminator(terminator);
            assert_eq!(matcher.get_line_terminator(), terminator);
            for byte in 0..=u8::MAX {
                check(&matcher, &[byte]);
            }
        }
    }

    // Independently tokenize line endings: CRLF is one token; lone CR/LF
    // each form a token. Boundaries fall before/after whole tokens.
    fn check_lines(matcher: &LookMatcher, bytes: &[u8]) {
        let mut starts = std::vec![0];
        let mut ends = std::vec![bytes.len()];
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte != 10 && byte != 13 {
                cursor += 1;
                continue;
            }
            ends.push(cursor);
            cursor += if byte == 13 && bytes.get(cursor + 1) == Some(&10) { 2 } else { 1 };
            starts.push(cursor);
        }
        for at in 0..=bytes.len() {
            assert_eq!(matcher.is_start(bytes, at), at == 0);
            assert_eq!(matcher.is_end(bytes, at), at == bytes.len());
            assert_eq!(matcher.is_start_crlf(bytes, at), starts.contains(&at));
            assert_eq!(matcher.is_end_crlf(bytes, at), ends.contains(&at));
            let terminator = matcher.get_line_terminator();
            let before = at.checked_sub(1).and_then(|i| bytes.get(i));
            assert_eq!(matcher.is_start_lf(bytes, at), at == 0 || before == Some(&terminator));
            assert_eq!(matcher.is_end_lf(bytes, at), at == bytes.len() || bytes.get(at) == Some(&terminator));
        }
    }

    #[test]
    fn line_boundaries_follow_independent_crlf_tokenization() {
        let matcher = LookMatcher::new();
        check_lines(&matcher, &[]);
        for first in 0..=u8::MAX {
            check_lines(&matcher, &[first]);
            for second in 0..=u8::MAX {
                check_lines(&matcher, &[first, second]);
            }
        }
        for terminator in [0, 10, 13, 95, 255] {
            let mut matcher = LookMatcher::new();
            matcher.set_line_terminator(terminator);
            for byte in 0..=u8::MAX {
                check_lines(&matcher, &[byte, 13, 10, byte]);
            }
        }
    }

    #[test]
    fn invalid_positions_retain_panics_and_short_circuit_behavior() {
        let matcher = LookMatcher::new();
        let methods = [
            LookMatcher::is_word_ascii, LookMatcher::is_word_ascii_negate,
            LookMatcher::is_word_start_ascii, LookMatcher::is_word_end_ascii,
            LookMatcher::is_word_start_half_ascii,
        ];
        for bytes in [&[][..], &b"a"[..], &b"_!"[..]] {
            for at in [bytes.len() + 1, usize::MAX] {
                for method in methods {
                    assert!(std::panic::catch_unwind(|| method(&matcher, bytes, at)).is_err());
                }
                assert!(matcher.is_word_end_half_ascii(bytes, at));
                assert!(!matcher.is_start(bytes, at));
                assert!(!matcher.is_end(bytes, at));
                for method in [LookMatcher::is_start_lf, LookMatcher::is_end_lf,
                    LookMatcher::is_start_crlf, LookMatcher::is_end_crlf] {
                    assert!(std::panic::catch_unwind(|| method(&matcher, bytes, at)).is_err());
                }
            }
        }
    }
}
