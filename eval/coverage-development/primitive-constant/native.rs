//! PRIVATE-SOURCE witness, not unchanged full-library execution.
//! oracle.py --prepare writes verified original spans. Compile using
//! RSTYLE_PRIVATE_SOURCE=/absolute/prepared.rs rustc --edition=2021
//! -C debug-assertions=on -C overflow-checks=on -C panic=unwind native.rs -o WITNESS.
use std::io::{self, BufRead};

include!(env!("RSTYLE_PRIVATE_SOURCE"));

fn main() {
    assert!(cfg!(debug_assertions));
    assert_eq!(usize::BITS, 64);
    std::panic::set_hook(Box::new(|_| {}));
    println!("PRIVATE-SOURCE debug_assertions=true overflow_checks=true width=64");
    for line in io::stdin().lock().lines() {
        let line = line.expect("request");
        let fields: Vec<_> = line.split_whitespace().collect();
        let result = match fields.as_slice() {
            ["32", m, factor, shift] => std::panic::catch_unwind(|| {
                u64::from(mul_shift_32(
                    m.parse().expect("u32"),
                    factor.parse().expect("u64"),
                    shift.parse().expect("i32"),
                ))
            }),
            ["64", m, lo, hi, shift] => std::panic::catch_unwind(|| {
                mul_shift_64(
                    m.parse().expect("u64"),
                    &(lo.parse().expect("u64"), hi.parse().expect("u64")),
                    shift.parse().expect("u32"),
                )
            }),
            _ => panic!("invalid request"),
        };
        match result {
            Ok(value) => println!("normal {value}"),
            Err(_) => println!("panic"),
        }
    }
}
