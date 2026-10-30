fn check(api: u8, input: &[u8], counts: &mut [u64; 7]) {
    let snapshot = input.to_vec();
    let (tag, bytes, message) = regex_automata::__rstyle_accel_parser_probe(api, input);
    let expected_tag = if api == 2 {
        0
    } else if api == 0 && input.len() < 4 {
        1
    } else if u16::from(input[0]) / 4 != 0 {
        2
    } else {
        0
    };
    assert_eq!(tag, expected_tag, "API={api}, input={input:?}");
    match expected_tag {
        0 => {
            let mut expected = [0; 8];
            for position in 0..4 {
                expected[position] = input[position];
            }
            assert_eq!(bytes, expected, "API={api}, input={input:?}");
            assert_eq!(message, "");
        }
        1 => assert_eq!(message, "accelerator"),
        2 => assert_eq!(message, "accelerator bytes cannot have length more than 3"),
        _ => unreachable!(),
    }
    assert_eq!(input, snapshot, "original input changed");
    let category = match (api, expected_tag) {
        (0, 1) => 0,
        (0, 2) => 1,
        (0, 0) => 2,
        (1, 2) => 3,
        (1, 0) => 4,
        (2, 0) => 5,
        _ => unreachable!(),
    };
    counts[category] += 1;
    counts[6] += 1;
}

fn slice_cases(prefix: [u8; 4], lengths: &[usize], counts: &mut [u64; 7]) {
    for offset in 0..4 {
        for &len in lengths {
            for tail in [0, 165, 255] {
                let mut backing = [tail; 16];
                for position in 0..4 {
                    backing[offset + position] = prefix[position];
                }
                let original = backing;
                check(0, &backing[offset..offset + len], counts);
                assert_eq!(backing, original, "surrounding input changed");
            }
        }
    }
}

fn direct_cases(prefix: [u8; 4], counts: &mut [u64; 7]) {
    check(1, &prefix, counts);
    // Invalid first bytes remain meaningful controls for the unchecked original.
    check(2, &prefix, counts);
}

fn main() {
    assert_eq!(usize::BITS, 64);
    let mut counts = [0; 7];
    for first in 0..=255 {
        for pattern in [0, 165, 255] {
            let prefix = [first, pattern, pattern, pattern];
            slice_cases(prefix, &[0, 1, 2, 3, 4, 5, 6, 7, 8], &mut counts);
            direct_cases(prefix, &mut counts);
        }
    }
    for position in 1..4 {
        for value in 0..=255 {
            for first in 0..4 {
                let mut prefix = [first, 17, 83, 249];
                prefix[position] = value;
                slice_cases(prefix, &[4, 8], &mut counts);
                direct_cases(prefix, &mut counts);
            }
        }
    }
    assert_eq!(counts, [36_864, 45_360, 74_448, 756, 3_084, 3_840, 164_352]);
    println!("{{\"short_errors\":{},\"slice_invalid_errors\":{},\"slice_successes\":{},\"array_invalid_errors\":{},\"array_successes\":{},\"unchecked_controls\":{},\"total\":{},\"debug_assertions\":{},\"mismatches\":0}}",
        counts[0], counts[1], counts[2], counts[3], counts[4], counts[5], counts[6], cfg!(debug_assertions));
}
