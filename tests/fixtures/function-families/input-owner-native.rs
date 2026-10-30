//! Link the unchanged complete regex-automata0.4.18 library with --extern.
use regex_automata::{Anchored, Input, PatternID, Span};
use std::io::{self, BufRead};

fn anchor(value: &str) -> Anchored {
    match value {
        "n" => Anchored::No,
        "y" => Anchored::Yes,
        "pmax" => Anchored::Pattern(PatternID::MAX),
        value => Anchored::Pattern(PatternID::must(
            value.strip_prefix('p').unwrap().parse().unwrap(),
        )),
    }
}

fn main() {
    let getters = std::env::args().any(|argument| argument == "--getters");
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 5);
        let haystack: Vec<u8> = if fields[0] == "-" {
            Vec::new()
        } else {
            fields[0]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        };
        let start = fields[1].parse::<usize>().unwrap();
        let end = fields[2].parse::<usize>().unwrap();
        let earliest = fields[4] == "1";
        let mut input = Input::new(&haystack);
        let default = input.get_earliest();
        input.set_earliest(earliest);
        input.set_span(Span { start, end });
        input.set_anchored(anchor(fields[3]));
        let configured = input.get_earliest();
        let span = input.get_span();
        let positions = (input.start(), input.end(), input.is_done());
        input.set_span(Span {
            start: 0,
            end: haystack.len(),
        });
        input.set_anchored(Anchored::No);
        let neighboring = input.get_earliest();
        input.set_earliest(!earliest);
        let toggled = input.get_earliest();
        input.set_earliest(earliest);
        let restored = input.get_earliest();
        if getters {
            println!(
                "{} {} {} {} {} {} {} {} {} {}",
                u8::from(default),
                u8::from(configured),
                u8::from(neighboring),
                u8::from(toggled),
                u8::from(restored),
                span.start,
                span.end,
                positions.0,
                positions.1,
                u8::from(positions.2)
            );
            continue;
        }
        println!(
            "{} {} {} {} {}",
            u8::from(default),
            u8::from(configured),
            u8::from(neighboring),
            u8::from(toggled),
            u8::from(restored)
        );
    }
}
