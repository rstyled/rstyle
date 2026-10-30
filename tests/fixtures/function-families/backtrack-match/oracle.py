#!/usr/bin/env python3
"""Freeze-check original bytes; optionally compile a standalone private-body witness."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

RUNNER = r'''
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Debug)]
enum Expected { Normal(usize, usize), Bounds(usize, usize) }

// A maximal equal suffix specification, rather than cursor-by-cursor mutation.
fn specification(a: &[u8], cur: usize, floor: usize, b: &[u8], candidate: usize) -> Expected {
    if candidate == 0 || cur <= floor { return Expected::Normal(cur, candidate); }
    if cur > a.len() { return Expected::Bounds(a.len(), cur - 1); }
    if candidate > b.len() { return Expected::Bounds(b.len(), candidate - 1); }
    let limit = candidate.min(cur - floor);
    let matched = a[..cur].iter().rev().zip(b[..candidate].iter().rev())
        .take(limit).take_while(|(left, right)| left == right).count();
    Expected::Normal(cur - matched, candidate - matched)
}

fn check(a: &[u8], initial_cur: usize, floor: usize, b: &[u8], initial_candidate: usize,
         counts: &mut [u64; 3]) {
    let expected = specification(a, initial_cur, floor, b, initial_candidate);
    let (mut cur, mut candidate) = (initial_cur, initial_candidate);
    // These are distinct local scalar objects, including when the shared views overlap.
    let actual = catch_unwind(AssertUnwindSafe(|| {
        backtrack_match(a, &mut cur, floor, b, &mut candidate)
    }));
    match expected {
        Expected::Normal(want_cur, want_candidate) => {
            assert!(actual.is_ok(), "unexpected panic: {initial_cur}/{initial_candidate}/{floor}");
            assert_eq!((cur, candidate), (want_cur, want_candidate));
            counts[0] += 1;
            if (cur, candidate) == (initial_cur, initial_candidate) { counts[2] += 1; }
        }
        Expected::Bounds(length, index) => {
            let payload = actual.expect_err("invalid evaluated index must panic");
            let text = payload.downcast_ref::<String>().map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied()).expect("bounds panic text");
            assert_eq!(text, format!("index out of bounds: the len is {length} but the index is {index}"),
                       "wrong first bounds access");
            assert_eq!((cur, candidate), (initial_cur, initial_candidate), "store before panic");
            counts[1] += 1;
        }
    }
}

fn cursors(length: usize) -> Vec<usize> {
    (0..=length + 1).chain([usize::MAX]).collect()
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut counts = [0u64; 3];
    for left_len in 0..=6 {
        for right_len in 0..=6 {
            for pattern in 0..4 {
                let a: Vec<u8> = (0..left_len).map(|i| match pattern {
                    0 => 7, 1 => (i % 3) as u8, _ => i as u8,
                }).collect();
                let b: Vec<u8> = (0..right_len).map(|i| match pattern {
                    0 => 7, 1 => (i % 3) as u8, 2 => i as u8, _ => 255 - i as u8,
                }).collect();
                let before = (a.clone(), b.clone());
                for cur in cursors(a.len()) {
                    for candidate in cursors(b.len()) {
                        for floor in [0, 1, left_len, left_len + 1, usize::MAX] {
                            check(&a, cur, floor, &b, candidate, &mut counts);
                        }
                    }
                }
                assert_eq!((&a, &b), (&before.0, &before.1));
            }
        }
    }
    // Offset and overlapping views over one immutable backing, in both directions.
    for pattern in 0..3 {
        let backing: Vec<u8> = (0..24).map(|i| match pattern {
            0 => 3, 1 => (i % 4) as u8, _ => i as u8,
        }).collect();
        let before = backing.clone();
        for (alo, ahi, blo, bhi) in [(0, 16, 4, 20), (4, 20, 0, 16),
                                    (3, 19, 3, 19), (7, 7, 2, 15)] {
            let (a, b) = (&backing[alo..ahi], &backing[blo..bhi]);
            for cur in cursors(a.len()) {
                for candidate in cursors(b.len()) {
                    for floor in [0, 1, a.len(), usize::MAX] {
                        check(a, cur, floor, b, candidate, &mut counts);
                    }
                }
            }
        }
        assert_eq!(backing, before);
    }
    // Both invalid: distinguish which access panics by different len/index messages.
    check(&[0, 1], 7, 0, &[0, 1, 2], 9, &mut counts);
    check(&[0, 1], 2, 0, &[0, 1, 2], 9, &mut counts);
    check(&[], usize::MAX, 0, &[], 0, &mut counts);
    check(&[], usize::MAX, usize::MAX, &[], usize::MAX, &mut counts);
    println!("{{\"normal\":{},\"bounds_panics\":{},\"unchanged_cursor_cases\":{},\"mismatches\":0}}",
             counts[0], counts[1], counts[2]);
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run", action="store_true", help="compile/run host private-body witness")
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / "selection.json").read_text())
    reference = selection["frozen_reference"]
    historical = json.loads((directory / reference["path"]).read_text())[reference["index"]]
    function = selection["function"]
    if historical != function or historical["sha256"] != reference["sha256"]:
        raise SystemExit("Frozen source selection differs from historical original")
    root = args.source_root or Path(selection["package"]["source_root"])
    original = (root / function["path"]).read_bytes()
    if original[function["start"]:function["end"]].decode() != function["source"]:
        raise SystemExit("Original selected source bytes/span disagree")
    print(json.dumps({"source_check": "pass", "historical_identity_reused": True,
                      "admission_evaluated": False, "scope": selection["oracle_scope"]}), flush=True)
    if not args.run:
        return
    rustc = Path(os.environ.get("RUSTC", str(Path.home() / ".cargo/bin/rustc")))
    with tempfile.TemporaryDirectory(prefix="rstyle-backtrack-oracle-") as temp:
        source = Path(temp) / "oracle.rs"
        executable = Path(temp) / "oracle"
        source.write_text(function["source"] + "\n" + RUNNER)
        subprocess.run([str(rustc), "+nightly-2026-09-30", "--edition=2021", "-C", "opt-level=0",
                        "-C", "overflow-checks=yes", "-C", "debug-assertions=yes", "-C", "panic=unwind",
                        str(source), "-o", str(executable)], check=True)
        subprocess.run([str(executable)], check=True)


if __name__ == "__main__":
    main()
