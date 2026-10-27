use super::*;

#[test]
fn decoded_byte_ranges_prove_exhaustiveness_only_for_the_complete_u8_domain() {
    let domain = Domain::new("u8", None).unwrap();
    for split in 1..=255u16 {
        let first = IntegerRange {
            start: Some("b'\\0'".into()),
            end: Some(format!("b'\\x{split:02x}'")),
            inclusive: false,
        };
        let second = IntegerRange {
            start: Some(format!("b'\\x{split:02x}'")),
            end: Some("b'\\xff'".into()),
            inclusive: true,
        };
        let low = domain.range(&first).unwrap();
        let high = domain.range(&second).unwrap();
        assert!(
            (0..=255).all(|byte| {
                (low.0..=low.1).contains(&byte) || (high.0..=high.1).contains(&byte)
            })
        );
        assert!(covers(&mut [low, high], domain.limits));
        assert!(!covers(&mut [low, (high.0 + 1, high.1)], domain.limits));
    }
    assert_eq!(domain.value("(b'\\xff')"), Some(255));
    for ty in ["bool", "i8", "u16", "usize"] {
        assert!(Domain::new(ty, Some(64)).unwrap().value("b'a'").is_none());
    }
}

#[test]
fn domain_boundaries_keep_all_128_bits_and_actual_pointer_width() {
    for (ty, width, low, high) in [
        ("bool", None, "false", "true"),
        ("i8", None, "-128", "127"),
        ("u8", None, "0", "255"),
        (
            "i128",
            None,
            "-170141183460469231731687303715884105728",
            "170141183460469231731687303715884105727",
        ),
        ("u128", None, "0", "340282366920938463463374607431768211455"),
        ("usize", Some(32), "0", "4294967295"),
        (
            "isize",
            Some(64),
            "-9223372036854775808",
            "9223372036854775807",
        ),
    ] {
        let domain = Domain::new(ty, width).unwrap();
        assert_eq!(domain.value(low), Some(domain.limits.0), "{ty}");
        assert_eq!(domain.value(high), Some(domain.limits.1), "{ty}");
    }
    assert!(Domain::new("usize", None).is_none());
    for (ty, value) in [
        ("u8", "-1"),
        ("u8", "256"),
        ("i8", "-129"),
        ("i8", "128"),
        ("bool", "1"),
    ] {
        assert!(Domain::new(ty, None).unwrap().value(value).is_none());
    }
}

#[test]
fn range_coverage_agrees_with_enumerated_small_domain_oracle() {
    let domain = Domain::new("u8", None).unwrap();
    for left in ["..4", "0..=127", "0..128", "..=255", "0..", "0x00..0x80_u8"] {
        for right in ["4..", "128..", "129..", "254..=255", "0..1"] {
            let ranges: Vec<_> = [left, right]
                .iter()
                .map(|source| {
                    domain
                        .range(&IntegerRange::from_source(source).unwrap())
                        .unwrap()
                })
                .collect();
            let expected = (0..=255).all(|value| {
                ranges
                    .iter()
                    .any(|&(start, end)| start <= value && value <= end)
            });
            assert_eq!(
                covers(&mut ranges.clone(), domain.limits),
                expected,
                "{left}, {right}"
            );
        }
    }
    assert!(!covers(&mut [(0, u128::MAX - 1)], (0, u128::MAX)));
    assert!(covers(&mut [(1, u128::MAX), (0, 0)], (0, u128::MAX)));
}

#[test]
fn signed_range_order_and_exclusive_bounds_preserve_gaps() {
    let domain = Domain::new("i8", None).unwrap();
    let mut ranges: Vec<_> = ["..-1", "0.."]
        .iter()
        .map(|source| {
            domain
                .range(&IntegerRange::from_source(source).unwrap())
                .unwrap()
        })
        .collect();
    assert!(!covers(&mut ranges, domain.limits));
    let minus_one = domain.value("- /* gap */ 1i8").unwrap();
    ranges.push((minus_one, minus_one));
    assert!(covers(&mut ranges, domain.limits));
    assert!(
        domain
            .range(&IntegerRange::from_source("..-128").unwrap())
            .is_none()
    );
}
