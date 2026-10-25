#!/usr/bin/env python3
"""XXH3 source checks; opt-in original public checked-index decode runtime oracle."""
import argparse
import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

RUNNER = r'''
use regex_automata::{PatternID, util::primitives::{SmallIndex, StateID}};

fn decoded(bytes: [u8; 4]) -> u64 {
    // Mathematical oracle: no native conversion API used for expected values.
    let mut value = 0u64;
    if cfg!(target_endian = "little") {
        for byte in bytes.iter().rev() { value = value * 256 + u64::from(*byte); }
    } else {
        for byte in bytes { value = value * 256 + u64::from(byte); }
    }
    value
}

fn bytes(mut value: u64) -> [u8; 4] {
    let mut out = [0; 4];
    for i in 0..4 {
        let slot = if cfg!(target_endian = "little") { i } else { 3-i };
        out[slot] = (value % 256) as u8;
        value /= 256;
    }
    out
}

fn check(input: [u8; 4], counts: &mut [usize; 2]) {
    let expected = decoded(input);
    let small = SmallIndex::from_ne_bytes(input);
    let pattern = PatternID::from_ne_bytes(input);
    // Same public semantics, but separate generated source/admission obligation.
    let state = StateID::from_ne_bytes(input);
    if expected <= 2_147_483_646 {
        assert_eq!(small.expect("independent valid interval").as_u32() as u64, expected);
        assert_eq!(pattern.expect("independent valid interval").as_u32() as u64, expected);
        assert_eq!(state.expect("independent valid interval").as_u32() as u64, expected);
        counts[0] += 1;
    } else {
        assert_eq!(small.expect_err("independent invalid interval").attempted(), expected);
        assert_eq!(pattern.expect_err("independent invalid interval").attempted(), expected);
        assert_eq!(state.expect_err("independent invalid interval").attempted(), expected);
        counts[1] += 1;
    }
    assert_eq!(diagnostic::as_usize(expected as u32) as u64, expected);
}

fn main() {
    assert_eq!(usize::BITS, 64, "declared selected native hosts are64-bit");
    assert_eq!(SmallIndex::MAX.as_u32(), 2_147_483_646);
    assert_eq!(PatternID::MAX.as_u32(), 2_147_483_646);
    assert_eq!(StateID::MAX.as_u32(), 2_147_483_646);
    let mut counts = [0; 2];
    for value in [0,1,2_147_483_645,2_147_483_646,2_147_483_647,
                  2_147_483_648,4_294_967_294,4_294_967_295] {
        check(bytes(value), &mut counts);
    }
    for input in [[0,1,128,255],[255,128,1,0],[1,2,3,4],[4,3,2,1],
                  [127,255,255,254],[254,255,255,127],[0,0,0,128],[128,0,0,0]] {
        check(input, &mut counts);
    }
    // Every16-bit pair at each adjacent position; additionally sparse endpoints.
    for value in 0u32..=65_535 {
        let lo=(value % 256) as u8;
        let hi=(value / 256) as u8;
        for input in [[lo,hi,0,0],[0,lo,hi,0],[0,0,lo,hi],[lo,0,0,hi]] {
            check(input, &mut counts);
        }
    }
    let mut sample=0x9e37_79b9u32;
    for _ in 0..8192 {
        sample=sample.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        check(bytes(u64::from(sample)), &mut counts);
    }
    println!("{{\"original_public_cases_per_api\":{},\"success\":{},\"error\":{},\"private_u32_diagnostic_cases\":{},\"debug_assertions\":{},\"native_endian\":\"{}\",\"mismatches\":0}}",
        counts[0]+counts[1],counts[0],counts[1],counts[0]+counts[1],cfg!(debug_assertions),
        if cfg!(target_endian="little") {"little"} else {"big"});
}

'''


def xxh3(data):
    library = ctypes.util.find_library("xxhash")
    if library is None:
        raise SystemExit("Source checks require system libxxhash XXH3_64bits; no SHA fallback")
    function = ctypes.CDLL(library).XXH3_64bits
    function.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    function.restype = ctypes.c_uint64
    return f"xxh3-64:{function(data, len(data)):016x}"


def verify_sources(selection, root):
    for value in selection["functions"] + selection["generated_members"] + selection["source_obligations"]:
        raw = (root / value["path"]).read_bytes()
        observed = raw[value["start"]:value["end"]]
        if observed != value["source"].encode() or xxh3(observed) != value["checksum"]:
            raise SystemExit("Original source/span/checksum differs: " + value["name"])
    for value in selection["context_files"]:
        raw = (root / value["path"]).read_bytes()
        if len(raw) != value["byte_length"] or xxh3(raw) != value["checksum"]:
            raise SystemExit("Complete original context differs: " + value["path"])


def diagnostic_source(selection):
    obligations = {value["name"]: value for value in selection["source_obligations"]}
    trait = obligations["complete private U32 trait"]["source"]
    implementation = obligations["complete U32 for u32 implementation"]["source"]
    wrapper = "pub fn as_usize(value: u32) -> usize { U32::as_usize(value) }"
    return f"mod diagnostic {{\n{trait}\n{implementation}\n{wrapper}\n}}\n{RUNNER}"


def run_native(selection, root, context_name, debug_mode):
    rustc = str(Path.home() / ".cargo/bin/rustc")
    version = subprocess.check_output([rustc, "+" + selection["build_selection"]["toolchain"], "-vV"], text=True)
    if "commit-hash: " + selection["build_selection"]["rustc_commit"] not in version:
        raise SystemExit("Native oracle compiler differs from frozen build")
    host = next(line.split(": ", 1)[1] for line in version.splitlines() if line.startswith("host: "))
    if host not in selection["build_selection"]["targets"]:
        raise SystemExit("Native oracle host is outside frozen build selection: " + host)
    modes = [True, False] if debug_mode == "both" else [debug_mode == "debug"]
    for context in selection["build_selection"]["contexts"]:
        if context_name != "both" and context["name"] != context_name:
            continue
        for debug in modes:
            with tempfile.TemporaryDirectory(prefix="rstyle-checked-decode-oracle-") as temporary:
                directory = Path(temporary)
                (directory / "src").mkdir()
                manifest = (
                    '[package]\nname="checked_decode_oracle"\nversion="0.0.0"\nedition="2021"\n'
                    '[workspace]\n[dependencies]\n'
                    f'regex-automata={{path={json.dumps(str(root))},default-features={str(context["default_features"]).lower()}}}\n'
                    '[profile.dev]\nopt-level=0\noverflow-checks=true\n'
                    f'debug-assertions={str(debug).lower()}\npanic="unwind"\n'
                )
                (directory / "Cargo.toml").write_text(manifest)
                (directory / "src/main.rs").write_text(diagnostic_source(selection))
                environment = dict(os.environ)
                for key in list(environment):
                    if key.startswith(("RSTYLE_", "CARGO_PROFILE_", "CARGO_TARGET_")) or key in {
                        "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET",
                    }:
                        environment.pop(key, None)
                environment["RUSTUP_TOOLCHAIN"] = selection["build_selection"]["toolchain"]
                cargo = os.environ.get("CARGO", str(Path.home() / ".cargo/bin/cargo"))
                print(json.dumps({"context": context["name"], "debug_assertions": debug,
                                  "scope": "original public decoders + copied private U32 diagnostic",
                                  "admission_evaluated": False}), flush=True)
                subprocess.run([cargo, "+nightly-2026-09-30", "run", "--offline", "--quiet",
                                "--manifest-path", str(directory / "Cargo.toml"),
                                "--target-dir", str(directory / "target")],
                               env=environment, check=True, timeout=180)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="coordinator schedules original native oracle")
    parser.add_argument("--context", choices=["default", "no-default", "both"], default="both")
    parser.add_argument("--debug-mode", choices=["debug", "release-branch", "both"], default="debug")
    args = parser.parse_args()
    selection = json.loads((Path(__file__).resolve().parent / "selection.json").read_text())
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, root)
    print(json.dumps({"source_check": "pass", "checksum": "XXH3-64", "ordinary_functions": len(selection["functions"]),
                      "generated_member_templates": len(selection["generated_members"]),
                      "admission_evaluated": False, "native_run": args.run_native}), flush=True)
    if args.run_native:
        run_native(selection, root, args.context, args.debug_mode)


if __name__ == "__main__":
    main()
