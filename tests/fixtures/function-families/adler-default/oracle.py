#!/usr/bin/env python3
"""XXH3-check original Adler default family; native semantic oracle is opt-in."""
import argparse
import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

RUNNER = r'''
use adler2::Adler32;

fn main() {
    // Independently specified Adler identity is 1, not an emitted graph value.
    let new = Adler32::new();
    let default = <Adler32 as Default>::default();
    assert_eq!(new.checksum(), 1);
    assert_eq!(default.checksum(), 1);
    // Getter is an already-delivered control, not a new target family.
    for sum in [0, 1, 0xffff, 0x1_0000, 0x1234_5678, u32::MAX] {
        assert_eq!(Adler32::from_checksum(sum).checksum(), sum);
    }
    assert_eq!(diagnostic::new_fields(), (1, 0));
    assert_eq!(diagnostic::default_fields(), (1, 0));
    // Familiar name is no authority for another selected implementation/body.
    #[derive(Debug, PartialEq)]
    struct Other(u16, u16);
    impl Default for Other { fn default() -> Self { Self(2, 3) } }
    assert_eq!(Other::default(), Other(2, 3));
    struct Shadow;
    impl Default for Shadow { fn default() -> Self { Self } }
    impl Shadow { fn default() -> u32 { 99 } }
    assert_eq!(Shadow::default(), 99);
    let _: Shadow = <Shadow as Default>::default();
    println!("{{\"public_original_cases\":8,\"copied_private_field_cases\":2,\"dispatch_controls\":3,\"mismatches\":0}}");
}
'''


def xxh3(data):
    library = ctypes.util.find_library("xxhash")
    if library is None:
        raise SystemExit("Source checks require system libxxhash with XXH3_64bits; no SHA fallback")
    function = ctypes.CDLL(library).XXH3_64bits
    function.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    function.restype = ctypes.c_uint64
    return f"xxh3-64:{function(data, len(data)):016x}"


def verify_sources(selection, root):
    for value in selection["functions"] + selection["source_contexts"] + list(selection["diagnostic_regions"].values()):
        raw = (root / value["path"]).read_bytes()
        source = raw[value["start"]:value["end"]]
        if value.get("complete_file") and value["end"] != len(raw):
            raise SystemExit("Complete source length changed: " + value["path"])
        if source != value["source"].encode() or xxh3(source) != value["checksum"]:
            raise SystemExit("Frozen original source/checksum changed: " + value["path"])


def diagnostic_source(selection):
    regions = selection["diagnostic_regions"]
    declaration = regions["declaration"]["source"]
    # Added sibling witness functions inspect fields without changing original
    # declaration/function bytes. This is not natural owning-context acceptance.
    new = selection["functions"][0]["source"]
    default = regions["default_impl"]["source"]
    witnesses = """
    pub fn new_fields() -> (u16, u16) { let value = Adler32::new(); (value.a, value.b) }
    pub fn default_fields() -> (u16, u16) { let value = <Adler32 as Default>::default(); (value.a, value.b) }
    """
    return f"mod diagnostic {{\n{declaration}\nimpl Adler32 {{\n{new}\n}}\n{default}\n{witnesses}\n}}\n{RUNNER}"


def run_native(selection, root, context_name):
    for context in selection["build_selection"]["contexts"]:
        if context_name != "both" and context["name"] != context_name:
            continue
        with tempfile.TemporaryDirectory(prefix="rstyle-adler-default-oracle-") as temporary:
            directory = Path(temporary)
            (directory / "src").mkdir()
            default = str(context["default_features"]).lower()
            manifest = (
                '[package]\nname="adler_default_oracle"\nversion="0.0.0"\nedition="2021"\n'
                '[workspace]\n[dependencies]\n'
                f'adler2={{path={json.dumps(str(root))},default-features={default}}}\n'
                '[profile.dev]\nopt-level=0\noverflow-checks=true\ndebug-assertions=true\npanic="unwind"\n'
            )
            (directory / "Cargo.toml").write_text(manifest)
            (directory / "src/main.rs").write_text(diagnostic_source(selection))
            cargo = os.environ.get("CARGO", str(Path.home() / ".cargo/bin/cargo"))
            print(json.dumps({"native_context": context["name"], "admission_evaluated": False}), flush=True)
            subprocess.run([cargo, "+nightly-2026-09-30", "run", "--offline", "--quiet",
                            "--manifest-path", str(directory / "Cargo.toml"),
                            "--target-dir", str(directory / "target")], check=True, timeout=120)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="coordinator-scheduled host execution")
    parser.add_argument("--context", choices=["default", "no-default", "both"], default="both")
    args = parser.parse_args()
    selection = json.loads((Path(__file__).resolve().parent / "selection.json").read_text())
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, root)
    print(json.dumps({"source_check": "pass", "checksum": "XXH3-64", "selected_functions": 2,
                      "admission_evaluated": False, "native_run": args.run_native}), flush=True)
    if args.run_native:
        run_native(selection, root, args.context)


if __name__ == "__main__":
    main()
