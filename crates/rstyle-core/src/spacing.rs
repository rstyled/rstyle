//! Shared spacing policy over original function source and statement counts.

/// Reports functions at the statement threshold without a blank source line.
/// The complete source includes the signature, matching the legacy rule.
pub fn is_finding(source: &str, top_statements: usize, minimum: usize) -> bool {
    minimum > 0 && top_statements >= minimum && !crate::lines::has_blank_line(source)
}

#[cfg(test)]
mod tests {
    use super::is_finding;

    const COMPACT: &str = "fn f() {\n    a();\n    b();\n    c();\n    d();\n    e();\n}";

    #[test]
    fn statement_threshold_is_inclusive() {
        assert!(!is_finding(COMPACT, 4, 5));
        assert!(is_finding(COMPACT, 5, 5));
        assert!(is_finding(COMPACT, 6, 5));
    }

    #[test]
    fn zero_minimum_disables_findings() {
        assert!(!is_finding(COMPACT, 0, 0));
        assert!(!is_finding(COMPACT, 5, 0));
        assert!(!is_finding(COMPACT, usize::MAX, 0));
    }

    #[test]
    fn empty_or_whitespace_only_lines_provide_spacing() {
        assert!(!is_finding("fn f() {\n    a();\n\n    b();\n}", 5, 5));
        assert!(!is_finding("fn f() {\n    a();\n \t \n    b();\n}", 5, 5));
        assert!(is_finding(
            "fn f() {\n    a();\n    // phase\n    b();\n}",
            5,
            5
        ));
    }

    #[test]
    fn blank_lines_in_the_signature_also_provide_spacing() {
        let source = "fn f(\n    a: u8,\n\n    b: u8,\n) {\n    a();\n    b();\n}";
        assert!(!is_finding(source, 5, 5));
    }
}
