use rstyle_pdg::assertions::Assertion;

#[test]
fn assertion_syntax_preserves_original_condition_positions() {
    for source in [
        "assert!(b)",
        "core::assert!( /* λ */ b,)",
        "debug_assert!({ let x = true; x })",
        "::std::debug_assert!((b))",
    ] {
        let assertion = Assertion::parse(source).unwrap();
        let span = assertion.condition_span(source, 0);
        let condition = &source[span.start as usize..span.end as usize];
        assert!(
            matches!(condition, "b" | "{ let x = true; x }" | "(b)"),
            "{condition}"
        );
    }
}

#[test]
fn assertion_comparison_macros_and_unregistered_paths_are_deferred() {
    for source in [
        "assert_eq!(x, y)",
        "custom::assert!(b)",
        "assert!()",
        "panic!()",
        "assert!(b);",
        "assert!(b, n)",
    ] {
        assert!(Assertion::parse(source).is_err(), "{source}");
    }
}
