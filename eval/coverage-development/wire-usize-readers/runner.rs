fn bytes(mut word: u32, width: usize) -> [u8; 4] {
    let mut result = [0; 4];
    for position in 0..width {
        let slot = if cfg!(target_endian = "little") {
            position
        } else {
            width - 1 - position
        };
        result[slot] = (word % 256) as u8;
        word /= 256;
    }
    result
}

fn words(width: usize) -> Vec<u32> {
    let mut values = if width == 2 {
        vec![0, 1, 255, 256, 65534, 65535]
    } else {
        vec![0, 1, 255, 256, 65535, 65536, 4294967294, 4294967295]
    };
    values.extend((0..width * 8).map(|bit| 1u32 << bit));
    let mut sample = 0x9E37_79B9u32;
    for _ in 0..128 {
        sample = sample.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        values.push(if width == 2 { sample % 65536 } else { sample });
    }
    assert_eq!(values.len(), if width == 2 { 150 } else { 168 });
    values
}

fn message(actual: &'static str, expected: &'static str) {
    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual.as_ptr(), expected.as_ptr());
}

fn full(api: u8, slice: &[u8], word: u32, width: usize, what: &'static str) {
    let (tag, value, consumed, _) = regex_automata::__rstyle_usize_reader_probe(api, slice, what);
    assert_eq!((tag, value, consumed), (0, u64::from(word), width));
}

fn full_inputs(messages: &[&'static str]) -> u64 {
    let mut count = 0;
    for (width, apis) in [(2, [0, 2]), (4, [1, 3])] {
        for word in words(width) {
            for offset in 0..4 {
                for extra in 0..5 {
                    for tail in [0, 0xA5, 0xFF] {
                        let mut backing = [tail; 16];
                        backing[offset..offset + width]
                            .copy_from_slice(&bytes(word, width)[..width]);
                        let original = backing;
                        for &what in messages {
                            for api in apis {
                                full(
                                    api,
                                    &backing[offset..offset + width + extra],
                                    word,
                                    width,
                                    what,
                                );
                                assert_eq!(backing, original, "original input changed");
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    count
}

fn exhaustive_u16() -> u64 {
    let mut count = 0;
    for word in 0..65536 {
        let input = bytes(word, 2);
        for api in [0, 2] {
            full(api, &input[..2], word, 2, "");
            count += 1;
        }
    }
    count
}

fn short_inputs(messages: &[&'static str]) -> u64 {
    let mut count = 0;
    for (width, apis) in [(2, [0, 2]), (4, [1, 3])] {
        for offset in 0..4 {
            for len in 0..width {
                for pattern in [0, 127, 128, 255] {
                    let backing = [pattern; 16];
                    let original = backing;
                    for &what in messages {
                        for api in apis {
                            let (tag, _, _, actual) = regex_automata::__rstyle_usize_reader_probe(
                                api,
                                &backing[offset..offset + len],
                                what,
                            );
                            assert_eq!(tag, 1);
                            message(actual, what);
                            assert_eq!(backing, original);
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    count
}

fn constructors(messages: &[&'static str]) -> u64 {
    let mut count = 0;
    for api in [4, 5] {
        for &what in messages {
            let (tag, _, _, actual) = regex_automata::__rstyle_usize_reader_probe(api, &[], what);
            assert_eq!(tag, api - 3);
            message(actual, what);
            count += 1;
        }
    }
    count
}

fn main() {
    assert_eq!(usize::BITS, 64);
    static LEFT: [u8; 5] = *b"field";
    static RIGHT: [u8; 5] = *b"field";
    let messages = [
        "",
        core::str::from_utf8(&LEFT).unwrap(),
        core::str::from_utf8(&RIGHT).unwrap(),
    ];
    assert_ne!(messages[1].as_ptr(), messages[2].as_ptr());
    assert_eq!(messages[1], messages[2]);
    let exhaustive = exhaustive_u16();
    let full = full_inputs(&messages);
    let short = short_inputs(&messages);
    let constructors = constructors(&messages);
    assert_eq!(
        [exhaustive, full, short, constructors],
        [131_072, 114_480, 576, 6]
    );
    println!("{{\"exhaustive_u16\":{},\"full_inputs\":{},\"short_errors\":{},\"constructor_controls\":{},\"total\":{},\"debug_assertions\":{},\"mismatches\":0}}",
        exhaustive, full, short, constructors, exhaustive + full + short + constructors, cfg!(debug_assertions));
}
