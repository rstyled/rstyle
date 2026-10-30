//! Link the unchanged complete regex-automata library with --extern.
use regex_automata::{Match, PatternID, Span};
use std::io::{self, BufRead};

fn main() {
    println!("width {}", usize::BITS);
    for line in io::stdin().lock().lines() {
        let line = line.expect("input line");
        let values: Vec<usize> = line
            .split_whitespace()
            .map(|value| value.parse().expect("unsigned input"))
            .collect();
        assert_eq!(values.len(), 3);

        let (start, end, pattern) = (values[0], values[1], values[2]);
        let span = Span { start, end };
        if start <= end {
            let matched = Match::new(PatternID::must(pattern), start..end);
            let saved = matched.span();
            println!(
                "{} {} {} {} {} {}",
                span.len(),
                u8::from(span.is_empty()),
                matched.len(),
                u8::from(matched.is_empty()),
                saved.start,
                saved.end
            );
        } else {
            // Inverted Span is valid. Public Match construction requires an
            // ordered range; do not forge private state to extend its domain.
            println!(
                "{} {} none none none none",
                span.len(),
                u8::from(span.is_empty())
            );
        }
    }
}
