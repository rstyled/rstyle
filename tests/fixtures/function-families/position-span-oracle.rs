//! Independent runtime expectations for the unchanged published natural family.
#[cfg(test)]
#[path = "position-span.rs"]
mod development;

#[cfg(test)]
mod updates {
    use super::development::{Position, Span};
    include!("span-updates.rs");
}

#[cfg(test)]
mod tests {
    use regex_syntax::ast::{Position, Span};

    #[test]
    fn value_updates_preserve_complete_opposite_endpoint_and_original() {
        let start = Position::new(3, 5, 7);
        let end = Position::new(11, 13, 17);
        let replacement = Position::new(19, 23, 29);
        let original = Span::new(start, end);
        assert_eq!(original.with_start(replacement), Span::new(replacement, end));
        assert_eq!(original.with_end(replacement), Span::new(start, replacement));
        assert_eq!(original, Span::new(start, end));
        let build = |p: Position| super::development::Position::new(p.offset, p.line, p.column);
        let value = super::development::Span::new(build(start), build(end));
        let updated = super::updates::updated_both(value, build(replacement), build(start));
        assert!(updated == super::development::Span::new(build(replacement), build(start)));
        let (saved, changed) = super::updates::retained(value, build(replacement));
        assert!(saved == value);
        assert!(changed == super::development::Span::new(build(replacement), build(end)));
        assert!(!super::updates::updated_empty(value, build(replacement)));
        assert!(super::updates::updated_empty(value, build(end)));
    }

    #[test]
    fn constructor_to_owned_getter_matches_independent_offsets() {
        for x in [0, 1, 7, usize::MAX] {
            for y in [0, 1, 11, usize::MAX] {
                let natural = Span::new(Position::new(x, 1, 2), Position::new(y, 1, 3));
                assert_eq!(natural.is_empty(), x == y);
                assert_eq!(super::development::composed(x, y), x == y);
            }
        }
    }

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
