//! Link the unchanged complete base640.23.1 library with --extern.
use std::io::{self, BufRead};

fn main() {
    println!("width {}", usize::BITS);
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 2);
        let bytes = fields[0].parse::<usize>().unwrap();
        let padding = match fields[1] {
            "0" => false,
            "1" => true,
            _ => panic!("invalid padding flag"),
        };
        match base64::encoded_len(bytes, padding) {
            Some(length) => println!("some {length}"),
            None => println!("none"),
        }
    }
}
