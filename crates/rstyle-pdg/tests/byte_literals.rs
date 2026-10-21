use rstyle_pdg::byte_literals::value;

#[test]
fn byte_escape_values_cover_the_entire_domain_without_accepting_other_literals() {
    for byte in 0..=u8::MAX {
        assert_eq!(value(&format!("b'\\x{byte:02x}'")), Some(byte));
    }
    for (source, expected) in [
        ("b'_'", 95),
        ("b'i'", 105),
        (r"b'\n'", 10),
        (r"b'\r'", 13),
        (r"b'\t'", 9),
        (r"b'\0'", 0),
        (r"b'\\'", 92),
        (r"b'\''", 39),
    ] {
        assert_eq!(value(source), Some(expected), "{source}");
    }
    for source in [
        "'a'",
        "b\"a\"",
        "95u8",
        "b'a'u8",
        "b'aa'",
        r"b'\u{61}'",
        "-b'a'",
    ] {
        assert_eq!(value(source), None, "{source}");
    }
}
