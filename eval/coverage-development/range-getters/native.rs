//! Link the unchanged complete regex-automata library with --extern.
use regex_automata::{Input, Match, PatternID, Span};
use std::io::{self, BufRead};

fn main() {
    println!("width {}", usize::BITS);
    // Only the three explicitly requested constructor-panic controls are caught.
    std::panic::set_hook(Box::new(|_| {}));
    for line in io::stdin().lock().lines() {
        let line = line.expect("input line");
        let mut fields = line.split_whitespace();
        let kind = fields.next().expect("kind");
        let values: Vec<usize> = fields
            .map(|value| value.parse().expect("unsigned input"))
            .collect();
        assert_eq!(values.len(), 3);
        let (start, end, extra) = (values[0], values[1], values[2]);
        match kind {
            "S" => {
                let span = Span { start, end };
                let view = span.range();
                let converted = core::ops::Range::<usize>::from(span);
                let reverse = Span::from(start..end);
                if start <= end {
                    let matched = Match::new(PatternID::must(extra), span).range();
                    println!(
                        "S {} {} {} {} {} {} {} {}",
                        view.start,
                        view.end,
                        converted.start,
                        converted.end,
                        reverse.start,
                        reverse.end,
                        matched.start,
                        matched.end
                    );
                } else {
                    println!(
                        "S {} {} {} {} {} {} none none",
                        view.start,
                        view.end,
                        converted.start,
                        converted.end,
                        reverse.start,
                        reverse.end
                    );
                }
            }
            "I" => {
                let haystack = vec![b'x'; extra];
                let mut input = Input::new(&haystack);
                let initial = input.get_range();
                input.set_span(Span { start, end });
                let selected = input.get_range();
                println!(
                    "I {} {} {} {}",
                    initial.start, initial.end, selected.start, selected.end
                );
            }
            "P" => {
                assert!(start > end);
                let rejected =
                    std::panic::catch_unwind(|| Match::new(PatternID::ZERO, Span { start, end }))
                        .is_err();
                println!("P {}", if rejected { "rejected" } else { "accepted" });
            }
            _ => panic!("unknown request kind"),
        }
    }
}
