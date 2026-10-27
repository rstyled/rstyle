// Appended beneath the original combinator module; calls unchanged private wrappers.
pub(crate) mod rstyle_digit_prefix_oracle {
    extern crate std;
    use super::rfc::iso8601::{hour, min};
    use std::vec;

    const DIGITS: [u8; 10] = *b"0123456789";

    fn expected(input: &[u8]) -> Option<u8> {
        let first = DIGITS.iter().position(|byte| Some(byte) == input.first())?;
        let second = DIGITS.iter().position(|byte| Some(byte) == input.get(1))?;
        Some((10 * first + second) as u8)
    }

    fn check(input: &[u8]) -> u64 {
        let before = input.to_vec();
        for original in [hour, min] {
            match (expected(input), original(input)) {
                (Some(value), Some(item)) => {
                    assert_eq!(item.1, value, "input={input:?}");
                    assert_eq!(item.0, &input[2..]);
                    assert_eq!(item.0.len(), input.len() - 2);
                    assert_eq!(item.0.as_ptr(), input.as_ptr().wrapping_add(2));
                }
                (None, None) => {}
                _ => panic!("wrong result classification: {input:?}"),
            }
        }
        assert_eq!(input, before);
        2
    }

    fn prefixes() -> u64 {
        let mut checks = check(&[]);
        for first in 0..=u8::MAX {
            checks += check(&[first]);
            for second in 0..=u8::MAX {
                checks += check(&[first, second]);
            }
        }
        checks
    }

    fn suffixes() -> u64 {
        let mut checks = 0;
        let variants: [&[u8]; 7] = [b"", b"9", b"000", b"\0", &[0xff], &[0x80, 0xc0, 0xff, 0], b"tail"];
        for first in DIGITS {
            for second in DIGITS {
                for suffix in 0..=u8::MAX {
                    checks += check(&[first, second, suffix]);
                }
                for suffix in variants {
                    let mut input = vec![first, second];
                    input.extend_from_slice(suffix);
                    checks += check(&input);
                }
            }
        }
        checks
    }

    fn pointers() -> u64 {
        let first = vec![b'9', b'9', 0xff];
        let second = first.clone();
        assert_ne!(first.as_ptr(), second.as_ptr());
        let checks = check(&first) + check(&second) + check(&[0, b'0', b'0', 255][1..]);
        for original in [hour, min] {
            let mut selected = first.as_slice();
            let saved = original(selected).unwrap();
            selected = &second;
            assert_eq!(saved.0.as_ptr(), first[2..].as_ptr());
            assert_ne!(saved.0.as_ptr(), selected[2..].as_ptr());
        }
        checks + 2
    }

    pub fn run() {
        let total = prefixes() + suffixes() + pointers();
        assert_eq!(total, 184_194);
        std::println!("RSTYLE_DIGIT_FORWARDER_ORACLE {{\"total_checks\":{total},\"mismatches\":0}}");
    }
}
