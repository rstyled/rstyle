// Append-only child in a byte-preserving whole-crate copy; originals stay unchanged.
pub(crate) mod rstyle_digit_prefix_oracle {
    extern crate std;
    use super::rfc::iso8601::{day, dayo, week};
    use std::vec;

    const DIGITS: [u8; 10] = *b"0123456789";

    fn expected(input: &[u8], width: usize) -> Option<u16> {
        let mut value = 0;
        for index in 0..width {
            let byte = input.get(index)?;
            let digit = DIGITS.iter().position(|candidate| candidate == byte)?;
            value = 10 * value + digit as u16;
        }
        if value == 0 { None } else { Some(value) }
    }

    fn suffix(actual: &[u8], input: &[u8], width: usize) {
        assert_eq!(actual, &input[width..]);
        assert_eq!(actual.len(), input.len() - width);
        assert_eq!(actual.as_ptr(), input.as_ptr().wrapping_add(width));
    }

    fn two(input: &[u8]) -> u64 {
        let before = input.to_vec();
        for original in [week, day] {
            match (expected(input, 2), original(input)) {
                (Some(value), Some(item)) => {
                    assert_eq!(u16::from(item.1.get()), value, "input={input:?}");
                    suffix(item.0, input, 2);
                }
                (None, None) => {}
                _ => panic!("wrong two-digit classification: {input:?}"),
            }
        }
        assert_eq!(input, before);
        2
    }

    fn three(input: &[u8]) -> u64 {
        let before = input.to_vec();
        match (expected(input, 3), dayo(input)) {
            (Some(value), Some(item)) => {
                assert_eq!(item.1.get(), value, "input={input:?}");
                suffix(item.0, input, 3);
            }
            (None, None) => {}
            _ => panic!("wrong three-digit classification: {input:?}"),
        }
        assert_eq!(input, before);
        1
    }

    fn exhaustive() -> u64 {
        let mut checks = two(&[]) + three(&[]);
        for first in 0..=u8::MAX {
            checks += two(&[first]) + three(&[first]);
            for second in 0..=u8::MAX {
                checks += two(&[first, second]) + three(&[first, second]);
            }
        }
        checks
    }

    fn suffixes() -> u64 {
        let mut checks = 0;
        let variants: [&[u8]; 7] = [
            b"",
            b"9",
            b"000",
            b"\0",
            &[255],
            &[128, 192, 255, 0],
            b"tail",
        ];
        for a in DIGITS {
            for b in DIGITS {
                for tail in 0..=u8::MAX {
                    checks += two(&[a, b, tail]);
                }
                for tail in variants {
                    let mut input = vec![a, b];
                    input.extend_from_slice(tail);
                    checks += two(&input);
                }
                for c in DIGITS {
                    for tail in variants {
                        let mut input = vec![a, b, c];
                        input.extend_from_slice(tail);
                        checks += three(&input);
                    }
                }
            }
        }
        checks
    }

    fn invalid() -> u64 {
        let mut checks = 0;
        for position in 0..3 {
            for byte in 0..=u8::MAX {
                if DIGITS.contains(&byte) {
                    continue;
                }
                for base in [*b"000", *b"001", *b"367", *b"999"] {
                    let mut input = vec![base[0], base[1], base[2], 0, 255];
                    input[position] = byte;
                    checks += three(&input);
                }
            }
        }
        checks
    }

    fn controls() -> u64 {
        // These are lexical parsers, not calendar validators.
        assert!(week(b"54").is_some());
        assert!(week(b"99").is_some());
        assert!(day(b"32").is_some());
        assert!(day(b"99").is_some());
        assert!(dayo(b"367").is_some());
        assert!(dayo(b"999").is_some());
        assert!(week(b"00").is_none());
        assert!(day(b"00").is_none());
        assert!(dayo(b"000").is_none());
        let first = vec![b'9', b'9', b'9', 255, 0];
        let second = first.clone();
        assert_ne!(first.as_ptr(), second.as_ptr());
        let mut checks = two(&first) + two(&second) + three(&first) + three(&second);
        checks += two(&[0, b'9', b'9', 255][1..]) + three(&[0, b'9', b'9', b'9', 255][1..]);
        for original in [week, day] {
            let mut selected = first.as_slice();
            let saved = original(selected).unwrap();
            selected = &second;
            assert_eq!(saved.0.as_ptr(), first[2..].as_ptr());
            assert_ne!(saved.0.as_ptr(), selected[2..].as_ptr());
        }
        let mut selected = first.as_slice();
        let saved = dayo(selected).unwrap();
        selected = &second;
        assert_eq!(saved.0.as_ptr(), first[3..].as_ptr());
        assert_ne!(saved.0.as_ptr(), selected[3..].as_ptr());
        checks + 9 + 3
    }

    pub fn run() {
        let total = exhaustive() + suffixes() + invalid() + controls();
        assert_eq!(total, 259_952);
        std::println!("RSTYLE_NONZERO_DIGIT_ORACLE {{\"total_checks\":{total},\"mismatches\":0}}");
    }
}
