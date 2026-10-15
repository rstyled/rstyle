//! Prebuild, then run separately from pipeline timing. JSON lines are raw samples.
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Instant};
use xxhash_rust::xxh3::Xxh3;

#[derive(Clone, Copy, serde::Serialize)]
enum Algorithm {
    Sha256,
    Xxh3,
}

fn checksum(algorithm: Algorithm, bytes: &[u8], chunk: usize) {
    match algorithm {
        Algorithm::Sha256 => {
            let mut hash = Sha256::new();
            for bytes in bytes.chunks(chunk) {
                hash.update(black_box(bytes));
            }
            black_box(hash.finalize());
        }
        Algorithm::Xxh3 => {
            let mut hash = Xxh3::new();
            for bytes in bytes.chunks(chunk) {
                hash.update(black_box(bytes));
            }
            black_box(hash.digest());
        }
    }
}

fn samples(algorithm: Algorithm, bytes: &[u8], chunk: usize) -> Vec<f64> {
    checksum(algorithm, bytes, chunk);
    (0..3)
        .map(|_| {
            let started = Instant::now();
            checksum(algorithm, bytes, chunk);
            started.elapsed().as_secs_f64()
        })
        .collect()
}

fn main() {
    let bytes: Vec<_> = (0..1024 * 1024).map(|index| (index % 251) as u8).collect();
    for chunk in [1, 16, 1024, 65536] {
        for algorithm in [Algorithm::Sha256, Algorithm::Xxh3] {
            let seconds = samples(algorithm, &bytes, chunk);
            println!(
                "{}",
                serde_json::json!({"algorithm":algorithm, "bytes":bytes.len(), "chunk":chunk, "seconds":seconds})
            );
        }
    }
}
