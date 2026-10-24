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
fn assertion_unsupported_messages_arities_and_paths_are_deferred() {
    for source in [
        "assert_eq!(x, y, \"message\")",
        "assert_eq!(x)",
        "debug_assert_eq!(x, y, 3)",
        "custom::assert!(b)",
        "assert!()",
        "panic!()",
        "assert!(b);",
        "assert!(b, n)",
    ] {
        assert!(Assertion::parse(source).is_err(), "{source}");
    }
}

#[test]
fn comparison_assertions_preserve_both_original_operand_ranges() {
    for source in [
        "assert_eq!(left, right)",
        "core::assert_eq!( /* λ */ left, right,)",
        "debug_assert_eq!({ left }, right)",
    ] {
        let assertion = Assertion::parse(source).unwrap();
        let left = assertion.condition_span(source, 0);
        let right = assertion.right_span(source, 0).unwrap();
        assert!(matches!(
            &source[left.start as usize..left.end as usize],
            "left" | "{ left }"
        ));
        assert_eq!(&source[right.start as usize..right.end as usize], "right");
        assert!(assertion.diagnostic.is_none());
    }
}
