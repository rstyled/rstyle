//! Link the unchanged complete regex-automata0.4.18 crate with --extern.
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 6);
        let source: Vec<u8> = if fields[0] == "-" {
            Vec::new()
        } else {
            fields[0]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        };
        // Remains live while observing source, so equal lengths cannot hide retargeting.
        let different: Vec<u8> = source.iter().map(|value| value ^ 0xFF).collect();
        let mut input = Input::new(&source);
        input.set_span(Span {
            start: fields[1].parse().unwrap(),
            end: fields[2].parse().unwrap(),
        });
        input.set_anchored(anchor(fields[3]));
        input.set_earliest(fields[4] == "1");
        let offset = if fields[5] == "max" {
            usize::MAX
        } else {
            fields[5].parse().unwrap()
        };
        let saved = input.haystack();
        let boundary = input.is_char_boundary(offset);
        let string_boundary = std::str::from_utf8(&source)
            .ok()
            .map(|s| s.is_char_boundary(offset));
        input.set_span(Span {
            start: 0,
            end: source.len(),
        });
        input.set_anchored(Anchored::No);
        input.set_earliest(fields[4] != "1");
        let result = input.haystack();
        let stable = saved.as_ptr() == source.as_ptr()
            && saved == source.as_slice()
            && result.as_ptr() == source.as_ptr()
            && result == source.as_slice();
        let distinct = source.is_empty() || result.as_ptr() != different.as_ptr();
        println!(
            "{} {} {} {} {} {}",
            u8::from(stable),
            result.len(),
            if result.is_empty() {
                "-".into()
            } else {
                hex(result)
            },
            u8::from(boundary),
            string_boundary.map_or("-".into(), |value| u8::from(value).to_string()),
            u8::from(distinct)
        );
    }
}
