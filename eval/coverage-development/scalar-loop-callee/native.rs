//! PRIVATE-SOURCE witness, not execution through the unchanged full ryu library.
//! oracle.py --prepare supplies the exact source-hashed private function spans.
//! Build with RSTYLE_PRIVATE_SOURCE=/absolute/path/to/prepared.rs rustc
//! --edition=2021 -C debug-assertions=on -C overflow-checks=on -C panic=unwind
//! native.rs -o /tmp/ryu-private-oracle
use std::io::{self, BufRead};

include!(env!("RSTYLE_PRIVATE_SOURCE"));

fn main() {
    assert!(
        cfg!(debug_assertions),
        "zero requires the selected debug assertion"
    );
    assert_eq!(usize::BITS, 64, "selected build contexts are 64-bit");
    std::panic::set_hook(Box::new(|_| {}));
    println!("PRIVATE-SOURCE debug_assertions=true width=64");
    for line in io::stdin().lock().lines() {
        let line = line.expect("request line");
        let mut fields = line.split_whitespace();
        let value: u32 = fields.next().expect("value").parse().expect("u32 value");
        let p: u32 = fields.next().expect("power").parse().expect("u32 power");
        assert!(fields.next().is_none(), "exactly two request fields");
        // Catch independently: zero must panic in both, including caller p=0.
        let factor = std::panic::catch_unwind(|| pow5factor_32(value));
        let predicate = std::panic::catch_unwind(|| multiple_of_power_of_5_32(value, p));
        match (factor, predicate) {
            (Ok(k), Ok(result)) => println!("normal {k} {result}"),
            (Err(_), Err(_)) => println!("panic panic"),
            _ => panic!("helper and caller disagree on exceptional outcome"),
        }
    }
}
