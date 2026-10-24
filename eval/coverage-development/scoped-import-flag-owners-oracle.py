"""Run independent flag-owner expectations in the complete original crate."""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
IDENTITIES = (
    (121083, 121133, "2c852f2f5c9bea703152abd735c471145825bde2aa203a16f5df2cb2ba52ad41"),
    (121214, 121267, "ce7eb5e80c13fb040592e0fdccf4e7d240c5b88b818d369177758fb1f3ab7edb"),
)
ORACLE = """
#[cfg(test)]
mod rstyle_flag_owner_oracle {
    use super::Epsilons;

    #[test]
    fn zero_is_the_only_empty_word_and_constructor_composes() {
        assert!(Epsilons::empty().is_empty());
        assert!(Epsilons(0).is_empty());
        assert!(!Epsilons(u64::MAX).is_empty());
        for bit in 0..64 {
            assert!(!Epsilons(1 << bit).is_empty());
        }
        for word in 0..=65535u64 {
            assert_eq!(Epsilons(word).is_empty(), word == 0);
            assert_eq!(Epsilons(word << 48).is_empty(), word == 0);
        }
    }
}
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", type=Path)
    args = parser.parse_args()
    source = (args.crate / "src/dfa/onepass.rs").read_bytes()
    for start, end, digest in IDENTITIES:
        if hashlib.sha256(source[start:end]).hexdigest() != digest:
            raise ValueError(f"Original source identity changed at {start}..{end}")
    with tempfile.TemporaryDirectory(prefix="rstyle-flag-owner-oracle-") as temp:
        destination = Path(temp) / "original"
        shutil.copytree(args.crate, destination, ignore=shutil.ignore_patterns("target", ".git"))
        (destination / "src/dfa/onepass.rs").write_bytes(source + b"\n" + ORACLE.encode())
        subprocess.run([
            "cargo", "test", "--locked", "--manifest-path", str(destination / "Cargo.toml"),
            "--target-dir", str(Path(temp) / "target"), "--lib", "rstyle_flag_owner_oracle",
        ], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
