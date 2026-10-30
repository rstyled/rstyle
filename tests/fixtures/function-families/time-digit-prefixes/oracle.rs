// Appended as a child of the original combinator module in a temporary crate copy.
pub(crate) mod rstyle_digit_prefix_oracle {
    extern crate std;

    use super::ExactlyNDigits;
    use std::vec;

    const DIGITS: [u8; 10] = *b"0123456789";
    const WEIGHTS: [u32; 3] = [1, 10, 100];

    // A table lookup and positional sum, independent of the original arithmetic.
    fn expected(input: &[u8], width: usize) -> Option<u32> {
        let prefix = input.get(..width)?;
        prefix
            .iter()
            .enumerate()
            .try_fold(0, |total, (index, byte)| {
                let digit = DIGITS.iter().position(|candidate| candidate == byte)?;
                Some(total + digit as u32 * WEIGHTS[width - index - 1])
            })
    }

    // Calls the unchanged original methods; no copied implementation is executed.
    fn original(input: &[u8], width: usize) -> Option<(&[u8], u32)> {
        match width {
            1 => ExactlyNDigits::<1>::parse(input).map(|item| (item.0, u32::from(item.1))),
            2 => ExactlyNDigits::<2>::parse(input).map(|item| (item.0, u32::from(item.1))),
            3 => ExactlyNDigits::<3>::parse(input).map(|item| (item.0, u32::from(item.1))),
            _ => panic!("unselected original width"),
        }
    }

    fn check(input: &[u8], width: usize) {
        let before = input.to_vec();
        let observed = original(input, width);
        match (expected(input, width), observed) {
            (None, None) => {}
            (Some(value), Some((remaining, actual))) => {
                assert_eq!(actual, value, "width={width}, input={input:?}");
                assert_eq!(remaining.len(), input.len() - width);
                assert_eq!(remaining, &input[width..]);
                assert_eq!(remaining.as_ptr(), input.as_ptr().wrapping_add(width));
            }
            (expected, actual) => panic!(
                "classification mismatch width={width}, input={input:?}, expected={expected:?}, actual={actual:?}"
            ),
        }
        assert_eq!(input, before, "original parser changed backing bytes");
    }

    fn check_widths(input: &[u8]) -> u64 {
        for width in 1..=3 {
            check(input, width);
        }
        3
    }

    fn exhaustive_prefixes() -> (u64, u64) {
        let mut one = 0;
        for byte in 0..=u8::MAX {
            one += check_widths(&[byte]);
        }
        let mut two = 0;
        for left in 0..=u8::MAX {
            for right in 0..=u8::MAX {
                two += check_widths(&[left, right]);
            }
        }
        (one, two)
    }

    fn valid_prefixes_and_suffixes() -> u64 {
        let suffixes: [&[u8]; 7] = [
            b"",
            b"9",
            b"000",
            b"\0",
            &[0xff],
            &[0x80, 0xc0, 0xff, 0],
            b"not a digit",
        ];
        let mut cases = 0;
        for value in 0..1000 {
            let decimal = std::format!("{value:03}");
            for suffix in suffixes {
                let mut input = decimal.as_bytes().to_vec();
                input.extend_from_slice(suffix);
                cases += check_widths(&input);
            }
        }
        // Every possible first suffix byte, including non-UTF8, for every width.
        for width in 1..=3 {
            for suffix in 0..=u8::MAX {
                let mut input = vec![b'9'; width];
                input.push(suffix);
                check(&input, width);
                cases += 1;
            }
        }
        cases
    }

    fn invalid_positions() -> u64 {
        let mut cases = 0;
        for width in 1..=3 {
            for baseline in [b'0', b'5', b'9'] {
                for position in 0..width {
                    for invalid in 0..=u8::MAX {
                        if DIGITS.contains(&invalid) {
                            continue;
                        }
                        let mut input = vec![baseline; width];
                        input[position] = invalid;
                        for suffix in [&b""[..], &[0xff, 0, b'9'][..]] {
                            let mut full = input.clone();
                            full.extend_from_slice(suffix);
                            assert!(expected(&full, width).is_none());
                            check(&full, width);
                            cases += 1;
                        }
                    }
                }
            }
        }
        cases
    }

    fn short_inputs() -> u64 {
        let mut cases = 0;
        for width in 1..=3 {
            for length in 0..width {
                for byte in 0..=u8::MAX {
                    let input = vec![byte; length];
                    assert!(expected(&input, width).is_none());
                    check(&input, width);
                    cases += 1;
                }
            }
        }
        cases
    }

    fn pointer_cases() -> u64 {
        let mut cases = 0;
        for width in 1..=3 {
            for suffix in [&b""[..], &[0, 0xff, b'7'][..]] {
                let mut left = vec![b'5'; width];
                left.extend_from_slice(suffix);
                let right = left.clone();
                assert_ne!(left.as_ptr(), right.as_ptr(), "distinct live allocations");
                check(&left, width);
                check(&right, width);
                let (left_rest, _) = original(&left, width).unwrap();
                let (right_rest, _) = original(&right, width).unwrap();
                assert_ne!(left_rest.as_ptr(), right_rest.as_ptr());

                let mut backing = vec![0xff; 3];
                backing.extend_from_slice(&left);
                backing.extend_from_slice(&[0x80, 0x81]);
                let window = &backing[3..3 + left.len()];
                check(window, width);
                let mut caller_input = window;
                let (saved_rest, _) = original(caller_input, width).unwrap();
                caller_input = &right;
                assert_eq!(saved_rest.as_ptr(), window.as_ptr().wrapping_add(width));
                assert_ne!(
                    saved_rest.as_ptr(),
                    caller_input.as_ptr().wrapping_add(width)
                );
                assert_eq!(saved_rest, suffix);
                assert_eq!(backing[0..3], [0xff; 3]);
                assert_eq!(&backing[3 + left.len()..], &[0x80, 0x81]);
                cases += 3;
            }
        }
        cases
    }

    pub(crate) fn run() {
        assert!(cfg!(debug_assertions));
        assert_eq!(usize::BITS, 64);
        let (one, two) = exhaustive_prefixes();
        let valid_suffix = valid_prefixes_and_suffixes();
        let invalid = invalid_positions();
        let short = short_inputs();
        let pointers = pointer_cases();
        assert_eq!(one, 256 * 3);
        assert_eq!(two, 65536 * 3);
        let total = one + two + valid_suffix + invalid + short + pointers;
        std::println!(
            "RSTYLE_DIGIT_PREFIX_ORACLE {{\"one_byte_inputs\":256,\"two_byte_inputs\":65536,\"valid_three_digit_prefixes\":1000,\"one_byte_checks\":{one},\"two_byte_checks\":{two},\"valid_and_suffix_checks\":{valid_suffix},\"invalid_position_checks\":{invalid},\"short_checks\":{short},\"pointer_checks\":{pointers},\"total_checks\":{total},\"mismatches\":0}}"
        );
    }
}
