//! Independent runtime expectations for the unchanged published natural family.
#[cfg(test)]
mod tests {
    use regex_syntax::ast::{Position, Span};

    #[test]
    fn constructors_preserve_distinct_components_and_complete_positions() {
        let start = Position::new(3, 5, 7);
        let end = Position::new(11, 13, 17);
        assert_eq!((start.offset, start.line, start.column), (3, 5, 7));
        let span = Span::new(start, end);
        assert_eq!((span.start.offset, span.start.line, span.start.column), (3, 5, 7));
        assert_eq!((span.end.offset, span.end.line, span.end.column), (11, 13, 17));
        let single = Span::splat(end);
        assert_eq!(single.start, end);
        assert_eq!(single.end, end);
    }

    #[test]
    fn getters_depend_only_on_their_declared_field_paths() {
        for start_bits in 0..8 {
            for end_bits in 0..8 {
                let start = Position::new(start_bits & 1, (start_bits >> 1) & 1, (start_bits >> 2) & 1);
                let end = Position::new(end_bits & 1, (end_bits >> 1) & 1, (end_bits >> 2) & 1);
                let span = Span::new(start, end);
                assert_eq!(span.is_empty(), (start_bits & 1) == (end_bits & 1));
                assert_eq!(span.is_one_line(), (start_bits & 2) == (end_bits & 2));
            }
        }
    }
}
