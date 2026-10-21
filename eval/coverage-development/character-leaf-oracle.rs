//! Standalone oracle linked against the unchanged regex-syntax 0.8.11 library.
//! Run as a Cargo binary with regex-syntax = "=0.8.11"; no copied target body.

const META_CODE_POINTS: [u32; 18] = [
    0x23, 0x24, 0x26, 0x28, 0x29, 0x2A, 0x2B, 0x2D, 0x2E, 0x3F, 0x5B, 0x5C,
    0x5D, 0x5E, 0x7B, 0x7C, 0x7D, 0x7E,
];

fn main() {
    let mut valid = 0usize;
    let mut positives = 0usize;
    for code_point in 0..=0x10FFFF {
        let Some(character) = char::from_u32(code_point) else {
            assert!((0xD800..=0xDFFF).contains(&code_point));
            continue;
        };
        let expected = META_CODE_POINTS.binary_search(&code_point).is_ok();
        assert_eq!(
            regex_syntax::is_meta_character(character),
            expected,
            "U+{code_point:04X}"
        );
        valid += 1;
        positives += usize::from(expected);
    }
    assert_eq!(valid, 0x110000 - 0x800);
    assert_eq!(positives, 18);
    assert!(char::from_u32(0x110000).is_none());
    assert_eq!('\\', '\u{005C}');
    assert_eq!('\n', '\u{000A}');
    assert_eq!('\x41', 'A');
    assert_eq!('\u{1F600}', '😀');
    assert!('\u{D7FF}' < '\u{E000}');
    assert!('\u{E000}' < '\u{10FFFF}');
    println!("checked {valid} valid scalars; {positives} metacharacters");
}
