"""Test original tagged byte units inside a disposable complete original crate.

This runtime oracle adds only a test module to a copy of regex-automata 0.4.18.
It neither measures admission nor substitutes a declaration or method body.
"""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
IDENTITIES = (
    (4985, 5053, "2433cddaafe16f0cc09ef07d8d3c6f9240f63a5bee12b515b979b420046b14d9"),
    (5523, 5842, "abfcfdd0fe617b67e4c03728c19fbe1312cc45249b787801746054d3f53111e3"),
    (5977, 6128, "a27ac16335fbe204e853b2665b5db25c033ade8be5978f7143c8d090995142f8"),
    (6302, 6469, "b019b70f74e1c40b13b7e6788b7f2b3584de2d870fc92e84c0342e3096f3e825"),
    (6688, 6858, "4dfbbd07c91e502bca13dca320768d302d9d1d8e3bd6ea859e567788862afb5f"),
    (7216, 7283, "f3eace4f01d5d7b096884d6f8c08e4de0d15abec00cfa825e738e76b66064fff"),
    (7039, 7135, "380abc4e459093f751245bd243296c420abbc176fda0777abea3de15fd0fc58b"),
    (11998, 12190, "c692de960c7c46267c5f6f7562f85dc065bcac35375dbd15a11a5d7d70509cf2"),
)



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", type=Path)
    args = parser.parse_args()
    source = (args.crate / "src/util/alphabet.rs").read_bytes()
    for start, end, digest in IDENTITIES:
        if hashlib.sha256(source[start:end]).hexdigest() != digest:
            raise ValueError(f"Original source identity changed at {start}..{end}")
    oracle = Path(__file__).with_suffix(".rs").read_bytes()
    with tempfile.TemporaryDirectory(prefix="rstyle-tagged-units-oracle-") as temp:
        destination = Path(temp) / "original"
        shutil.copytree(args.crate, destination, ignore=shutil.ignore_patterns("target", ".git"))
        (destination / "src/util/alphabet.rs").write_bytes(source + b"\n" + oracle)
        subprocess.run([
            "cargo", "test", "--locked", "--manifest-path", str(destination / "Cargo.toml"),
            "--target-dir", str(Path(temp) / "target"), "--lib", "rstyle_tagged_units_oracle",
        ], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
