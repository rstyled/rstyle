#!/usr/bin/env python3
"""XXH3 source checks; opt-in original public checked-index runtime oracle."""
import argparse
import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

RUNNER = r'''
use regex_automata::{PatternID, util::primitives::SmallIndex};

fn check(index: usize, counts: &mut [usize; 2]) {
    // Independent interval specification, fixed before evaluating admission.
    const EXPECTED_MAX: usize = 2_147_483_646;
    // Current pinned implementation controls for the newly connected reads.
    // The unchecked API promises no logical range invariant; specify its full
    // observed integer behavior without assuming index <= EXPECTED_MAX.
    let raw = SmallIndex::new_unchecked(index);
    let low = (index as u64) % 4_294_967_296;
    let signed = if low < 2_147_483_648 { low as i64 } else { low as i64 - 4_294_967_296 };
    assert_eq!(raw.as_usize() as u64, low);
    assert_eq!(raw.as_u64(), low);
    assert_eq!(raw.as_u32() as u64, low);
    assert_eq!(raw.as_i32() as i64, signed);
    assert_eq!(raw.one_more() as u64, low + 1);
    let small = SmallIndex::new(index);
    let via_trait = <SmallIndex as TryFrom<usize>>::try_from(index);
    let pattern = PatternID::new(index);
    if index <= EXPECTED_MAX {
        let value = small.expect("index in independently specified interval");
        let trait_value = via_trait.expect("same interval via TryFrom");
        let wrapped = pattern.expect("same interval via generated constructor");
        assert_eq!(value.as_usize(), index);
        assert_eq!(trait_value.as_usize(), index);
        assert_eq!(wrapped.as_usize(), index);
        assert_eq!(format!("{value:?}"), format!("SmallIndex({index})"));
        assert_eq!(format!("{wrapped:?}"), format!("PatternID({index})"));
        counts[0] += 1;
    } else {
        let error = small.expect_err("index above independently specified interval");
        let trait_error = via_trait.expect_err("same failing interval via TryFrom");
        let wrapped = pattern.expect_err("same failing interval via generated constructor");
        assert_eq!(error.attempted(), index as u64);
        assert_eq!(trait_error.attempted(), index as u64);
        assert_eq!(wrapped.attempted(), index as u64);
        assert_eq!(format!("{error:?}"), format!("SmallIndexError {{ attempted: {index} }}"));
        assert_eq!(format!("{wrapped:?}"),
                   format!("PatternIDError(SmallIndexError {{ attempted: {index} }})"));
        counts[1] += 1;
    }
    // Private Usize trait copied verbatim, including BOTH debug/release bodies.
    // This control does not certify original private helper source ownership.
    assert_eq!(diagnostic::as_u64(index), index as u64);
}

fn main() {
    assert_eq!(usize::BITS, 64, "native oracle is declared for selected 64-bit hosts");
    assert_eq!(SmallIndex::MAX.as_usize(), 2_147_483_646);
    assert_eq!(PatternID::MAX.as_usize(), 2_147_483_646);
    let mut counts = [0usize; 2];
    let boundary = [0, 1, 2, 2_147_483_645, 2_147_483_646, 2_147_483_647,
                    u32::MAX as usize, u32::MAX as usize + 1, usize::MAX - 1, usize::MAX];
    for index in boundary { check(index, &mut counts); }
    // Deterministic full-width samples plus narrow samples ensure both variants.
    let mut state = 0x6a09_e667_f3bc_c909u64;
    for _ in 0..4096 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        check(state as usize, &mut counts);
        check((state as u32 >> 1) as usize, &mut counts);
    }
    println!("{{\"original_public_cases\":{},\"success\":{},\"error\":{},\"private_usize_diagnostic_cases\":{},\"debug_assertions\":{},\"mismatches\":0}}",
             counts[0]+counts[1], counts[0], counts[1], counts[0]+counts[1], cfg!(debug_assertions));
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
    trait = obligations["complete private Usize trait"]["source"]
    implementation = obligations["complete Usize for usize implementation"]["source"]
    wrapper = "pub fn as_u64(value: usize) -> u64 { Usize::as_u64(value) }"
    return f"mod diagnostic {{\n{trait}\n{implementation}\n{wrapper}\n}}\n{RUNNER}"


def run_native(selection, root, context_name, debug_mode):
    modes = [True, False] if debug_mode == "both" else [debug_mode == "debug"]
    for context in selection["build_selection"]["contexts"]:
        if context_name != "both" and context["name"] != context_name:
            continue
        for debug in modes:
            with tempfile.TemporaryDirectory(prefix="rstyle-checked-index-oracle-") as temporary:
                directory = Path(temporary)
                (directory / "src").mkdir()
                manifest = (
                    '[package]\nname="checked_index_oracle"\nversion="0.0.0"\nedition="2021"\n'
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
                                  "scope": "original public constructors + copied private Usize diagnostic",
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
