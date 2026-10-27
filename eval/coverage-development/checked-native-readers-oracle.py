"""Test original checked readers inside a disposable complete original crate.

This runtime oracle adds only a test module to a copy of regex-automata 0.4.18.
It neither measures admission nor substitutes an error type or helper body.
"""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
IDENTITIES = (
    (27879, 28093, "3707b6f20da7d8dd8764c106dabe5d335917c5ff6ddd5863c5358a510fd8794a"),
    (28481, 28695, "7fbbabf9a4fdb9c6f687e3fafdbe23a67db991bb5f25fecb24ac7664b1c9b1b6"),
    (29085, 29304, "54df143a143a2b6ba12cd335a957ad2200e154dda14a3706cef58232c86f26e4"),
    (30713, 30965, "d2fb054c313fd376ce0c006238e49f50aa37e49ec84807fba1b16dcb8cd53f19"),
    (6900, 7050, "294b8c2c73d68e637c40e58c668e2d71294f91d476fba37ea969a3d3fae5ad66"),
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", type=Path)
    args = parser.parse_args()
    source = (args.crate / "src/util/wire.rs").read_bytes()
    for start, end, digest in IDENTITIES:
        if hashlib.sha256(source[start:end]).hexdigest() != digest:
            raise ValueError(f"Original source identity changed at {start}..{end}")
    oracle = Path(__file__).with_suffix(".rs").read_bytes()
    with tempfile.TemporaryDirectory(prefix="rstyle-checked-native-oracle-") as temp:
        destination = Path(temp) / "original"
        shutil.copytree(args.crate, destination, ignore=shutil.ignore_patterns("target", ".git"))
        (destination / "src/util/wire.rs").write_bytes(source + b"\n" + oracle)
        subprocess.run([
            "cargo", "test", "--locked", "--manifest-path", str(destination / "Cargo.toml"),
            "--target-dir", str(Path(temp) / "target"), "--lib", "rstyle_checked_native_oracle",
        ], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
