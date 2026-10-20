"""Test selected original byte helpers inside a disposable complete original crate.

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

ACCEL_IDENTITIES = (
    (18618, 18687, "42d2bcaa66823a8ad482f6d8abd981f0d91cd19b3e4c0e8486de74d7ee3f5f3f"),
    (18803, 18865, "78d4124cda0a33ad8ad19db5c2b488457c1bb820179c97266b811a0bcc3d2d42"),
    (19001, 19074, "adc9be3867dec592d18a4ff056241b1a08d57d3629b54482c4dc011525cae7c1"),
)



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", type=Path)
    parser.add_argument("--family", choices=["tagged-units", "borrowed-accel"], default="tagged-units")
    args = parser.parse_args()
    relative, identities, oracle_path, test_filter = (
        ("src/dfa/accel.rs", ACCEL_IDENTITIES,
         Path(__file__).with_name("borrowed-accel-views-oracle.rs"), "rstyle_borrowed_accel_oracle")
        if args.family == "borrowed-accel" else
        ("src/util/alphabet.rs", IDENTITIES, Path(__file__).with_suffix(".rs"), "rstyle_tagged_units_oracle")
    )
    source = (args.crate / relative).read_bytes()
    for start, end, digest in identities:
        if hashlib.sha256(source[start:end]).hexdigest() != digest:
            raise ValueError(f"Original source identity changed at {start}..{end}")
    oracle = oracle_path.read_bytes()
    with tempfile.TemporaryDirectory(prefix="rstyle-tagged-units-oracle-") as temp:
        destination = Path(temp) / "original"
        shutil.copytree(args.crate, destination, ignore=shutil.ignore_patterns("target", ".git"))
        (destination / relative).write_bytes(source + b"\n" + oracle)
        subprocess.run([
            "cargo", "test", "--locked", "--manifest-path", str(destination / "Cargo.toml"),
            "--target-dir", str(Path(temp) / "target"), "--lib", test_filter,
        ], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
