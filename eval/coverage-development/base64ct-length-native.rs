//! Link unchanged full base64ct1.7.3 and base640.23.1 packages with --extern.
//! Optional original_helper mode includes only the exact frozen helper bytes;
//! that mode is a source witness, not execution inside the original package.
#[cfg(not(original_helper))]
use base64ct::{Base64, Base64Unpadded, Encoding};
use std::io::{self, BufRead};

#[cfg(original_helper)]
include!(env!("BASE64CT_LENGTH_ORIGINAL"));

fn option(value: Option<usize>) -> String {
    value.map_or("none".into(), |value| format!("some:{value}"))
}

fn main() {
    println!("width:{}", usize::BITS);
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut fields = line.split_whitespace();
        let n: usize = fields.next().unwrap().parse().unwrap();
        let padded = fields.next().unwrap() == "1";
        assert!(fields.next().is_none());
        #[cfg(original_helper)]
        let ct = option(encoded_len_inner(n, padded));
        #[cfg(not(original_helper))]
        let ct = {
            assert!(n <= 65536, "public API requires an actual borrowed slice");
            let source = vec![0u8; n];
            let value = if padded {
                Base64::encoded_len(&source)
            } else {
                Base64Unpadded::encoded_len(&source)
            };
            format!("some:{value}")
        };
        println!("{ct} {}", option(base64::encoded_len(n, padded)));
    }
}
