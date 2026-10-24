use rstyle_pdg::scalar_patterns::IntegerRange;

#[test]
fn integer_range_bounds_preserve_signed_values_and_upper_inclusivity() {
    for (source, start, end, inclusive) in [
        ("..0", None, Some("0"), false),
        ("..=0", None, Some("0"), true),
        ("0..", Some("0"), None, false),
        ("-128..127", Some("-128"), Some("127"), false),
        ("0x80..=0xff_u16", Some("0x80"), Some("0xff_u16"), true),
        ("- /* λ */ 5 ..= 3", Some("- /* λ */ 5"), Some("3"), true),
    ] {
        let range = IntegerRange::from_source(source).unwrap();
        assert_eq!(range.start.as_deref(), start, "{source}");
        assert_eq!(range.end.as_deref(), end, "{source}");
        assert_eq!(range.inclusive, inclusive, "{source}");
    }
}

#[test]
fn noninteger_or_unresolved_ranges_do_not_become_literal_ranges() {
    for source in [
        "LOW..=HIGH",
        "'a'..='z'",
        "false..=true",
        "_",
        "..",
        "(1..=3)",
    ] {
        assert!(IntegerRange::from_source(source).is_err(), "{source}");
    }
    let range =
        IntegerRange::from_source("0..=340282366920938463463374607431768211455u128").unwrap();
    assert_eq!(
        serde_json::from_str::<IntegerRange>(&range.text()).unwrap(),
        range
    );
}
