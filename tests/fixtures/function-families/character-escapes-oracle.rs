//! Run against unchanged regex-syntax = "=0.8.11"; no copied target body.

const ESCAPEABLE_INTERVALS: [(u32, u32); 6] = [
    (0, 47),
    (58, 59),
    (61, 61),
    (63, 64),
    (91, 96),
    (123, 127),
];

fn main() {
    let mut valid = 0usize;
    let mut positives = 0usize;
    for code_point in 0..=0x10FFFF {
        let Some(character) = char::from_u32(code_point) else {
            assert!((0xD800..=0xDFFF).contains(&code_point));
            continue;
        };
        let expected = ESCAPEABLE_INTERVALS
            .iter()
            .any(|&(first, last)| first <= code_point && code_point <= last);
        assert_eq!(
            regex_syntax::is_escapeable_character(character),
            expected,
            "U+{code_point:04X}"
        );
        valid += 1;
        positives += usize::from(expected);
    }
    assert_eq!(valid, 1_112_064);
    assert_eq!(positives, 64);
    assert_eq!(
        ESCAPEABLE_INTERVALS
            .iter()
            .map(|&(first, last)| last - first + 1)
            .sum::<u32>(),
        64
    );
    assert!(char::from_u32(0x110000).is_none());
    println!("checked {valid} valid scalars; {positives} escapeable characters");
}
