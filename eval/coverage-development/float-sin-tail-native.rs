//! Compile RSTYLE_K_SIN_SOURCE as the unchanged complete original source file.
mod original {
    include!(env!("RSTYLE_K_SIN_SOURCE"));
}

use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let values: Vec<_> = line.split_whitespace().collect();
        assert_eq!(values.len(), 3);
        let x = f64::from_bits(u64::from_str_radix(values[0], 16).unwrap());
        let y = f64::from_bits(u64::from_str_radix(values[1], 16).unwrap());
        println!(
            "{:016x}",
            original::k_sin(x, y, values[2].parse::<i32>().unwrap()).to_bits()
        );
    }
}
