//! Independent observations of the unchanged registry constructor and From impl.
use regex_automata::{Anchored, Input};
use std::cell::Cell;

fn assert_input(input: Input<'_>, expected: &[u8]) {
    assert_eq!(input.haystack(), expected);
    assert_eq!(input.haystack().as_ptr(), expected.as_ptr());
    assert_eq!(input.start(), 0);
    assert_eq!(input.end(), expected.len());
    assert_eq!(input.get_span().start, 0);
    assert_eq!(input.get_span().end, expected.len());
    assert_eq!(input.get_anchored(), Anchored::No);
    assert!(!input.get_earliest());
    assert!(!input.is_done());
}

fn check_bytes(bytes: &[u8]) {
    assert_input(Input::new::<[u8]>(bytes), bytes);
    assert_input(<Input<'_> as From<&[u8]>>::from(bytes), bytes);
}

fn check_text(text: &str) {
    assert_input(Input::new::<str>(text), text.as_bytes());
    assert_input(<Input<'_> as From<&str>>::from(text), text.as_bytes());
}

struct Switching<'a> {
    calls: Cell<usize>,
    first: &'a [u8],
    later: &'a [u8],
}

impl AsRef<[u8]> for Switching<'_> {
    fn as_ref(&self) -> &[u8] {
        let calls = self.calls.get();
        self.calls.set(calls + 1);
        if calls == 0 {
            self.first
        } else {
            self.later
        }
    }
}

fn check_effectful_conversion() {
    let first = [0, 0x80, 0xff];
    let later = [9];
    for from in [false, true] {
        let source = Switching {
            calls: Cell::new(0),
            first: &first,
            later: &later,
        };
        let input = if from {
            Input::from(&source)
        } else {
            Input::new(&source)
        };
        assert_input(input, &first);
        assert_eq!(source.calls.get(), 1);
    }
}

struct Static;

impl AsRef<[u8]> for Static {
    fn as_ref(&self) -> &[u8] {
        b"unrelated static storage"
    }
}

fn check_static_conversion() {
    assert_input(Input::new(&Static), b"unrelated static storage");
    assert_input(Input::from(&Static), b"unrelated static storage");
}

struct Panicking(Cell<usize>);

impl AsRef<[u8]> for Panicking {
    fn as_ref(&self) -> &[u8] {
        self.0.set(self.0.get() + 1);
        panic!("independent conversion panic");
    }
}

fn check_panicking_conversion() {
    for from in [false, true] {
        let source = Panicking(Cell::new(0));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if from {
                Input::from(&source)
            } else {
                Input::new(&source)
            }
        }));
        assert!(result.is_err());
        assert_eq!(source.0.get(), 1);
    }
}

fn main() {
    assert!(!std::mem::needs_drop::<Input<'_>>());
    let all: Vec<_> = (0..=255).collect();
    let other = all.clone();
    assert_ne!(all.as_ptr(), other.as_ptr());
    let mut byte_cases = 0;
    for storage in [&all, &other] {
        for start in 0..=storage.len() {
            for end in start..=storage.len() {
                check_bytes(&storage[start..end]);
                byte_cases += 1;
            }
        }
    }
    let texts = ["", "ascii", "\0", "é", "雪", "🦀", "a\0é雪🦀z"];
    let mut text_cases = 0;
    for text in texts {
        for start in 0..=text.len() {
            for end in start..=text.len() {
                if text.is_char_boundary(start) && text.is_char_boundary(end) {
                    check_text(&text[start..end]);
                    text_cases += 1;
                }
            }
        }
    }
    check_effectful_conversion();
    check_static_conversion();
    check_panicking_conversion();
    println!("{{\"byte_cases\":{byte_cases},\"text_cases\":{text_cases},\"constructor_observations\":{},\"effectful_checks\":2,\"static_origin_checks\":2,\"expected_panics\":2,\"mismatches\":0}}", 2 * (byte_cases + text_cases));
}
