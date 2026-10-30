"""Run independent byte membership expectations in the complete original crate."""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
IDENTITIES = (
    (28176, 28257, "95c66f772604235239ec20e74f5dd282c0f1066a69db322361fac0893858e4a0"),
    (28903, 29079, "3a795ab4f5e2bb7ef972473f37ebcf6e37569964e4b95a147986a2a57307c19d"),
)
ORACLE = """
#[cfg(test)]
mod rstyle_byte_membership_oracle {
    use super::{BitSet, ByteSet};

    #[test]
    fn empty_contains_no_byte() {
        for byte in 0..=255u8 {
            assert!(!ByteSet::empty().contains(byte));
        }
    }

    #[test]
    fn every_singleton_distinguishes_every_query() {
        for selected in 0..=255usize {
            let mut words = [0u128; 2];
            words[selected / 128] = 1u128 << (selected % 128);
            let set = ByteSet { bits: BitSet(words) };
            for query in 0..=255u8 {
                assert_eq!(set.contains(query), usize::from(query) == selected);
            }
        }
    }

    #[test]
    fn independently_accumulated_membership_matches_dense_words() {
        for seed in 0..64usize {
            let mut expected = [false; 256];
            let mut words = [0u128; 2];
            for index in 0..256usize {
                expected[index] = (index * 17 + seed * 13) % 31 < 15;
                words[index / 128] |= u128::from(expected[index]) << (index % 128);
            }
            let set = ByteSet { bits: BitSet(words) };
            for query in 0..=255u8 {
                assert_eq!(set.contains(query), expected[usize::from(query)]);
            }
        }
        let full = ByteSet { bits: BitSet([u128::MAX; 2]) };
        for query in 0..=255u8 { assert!(full.contains(query)); }
    }
}
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", type=Path)
    args = parser.parse_args()
    source = (args.crate / "src/util/alphabet.rs").read_bytes()
    for start, end, digest in IDENTITIES:
        if hashlib.sha256(source[start:end]).hexdigest() != digest:
            raise ValueError(f"Original source identity changed at {start}..{end}")
    with tempfile.TemporaryDirectory(prefix="rstyle-byte-membership-oracle-") as temp:
        destination = Path(temp) / "original"
        shutil.copytree(args.crate, destination, ignore=shutil.ignore_patterns("target", ".git"))
        (destination / "src/util/alphabet.rs").write_bytes(source + b"\n" + ORACLE.encode())
        subprocess.run([
            "cargo", "test", "--locked", "--manifest-path", str(destination / "Cargo.toml"),
            "--target-dir", str(Path(temp) / "target"), "--lib", "rstyle_byte_membership_oracle",
        ], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
