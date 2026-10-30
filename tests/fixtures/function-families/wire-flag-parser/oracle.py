#!/usr/bin/env python3
"""Source-first Flags parser audit; opt-in append-only whole-crate native oracle."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


# Appended inside the original dense module; no production body is copied.
DENSE = r'''
#[doc(hidden)]
pub(crate) fn __rstyle_wire_flag_parser_oracle() -> [u64; 3] {
    fn prefix(mut word: u32) -> [u8; 4] {
        let mut bytes = [0; 4];
        for position in 0..4 {
            let slot = if cfg!(target_endian = "little") { position } else { 3 - position };
            bytes[slot] = (word % 256) as u8;
            word /= 256;
        }
        bytes
    }
    fn check(bytes: &[u8], word: u32) -> bool {
        match Flags::from_bytes(bytes) {
            Err(error) => {
                assert!(bytes.len() < 4, "unexpected complete-input error");
                crate::util::wire::__rstyle_wire_flag_parser_error(error);
                false
            }
            Ok((flags, consumed)) => {
                assert!(bytes.len() >= 4, "short input admitted");
                // Independent arithmetic bits, not the source masks/shifts.
                assert_eq!(flags.has_empty, word % 2 != 0);
                assert_eq!(flags.is_utf8, (word / 2) % 2 != 0);
                assert_eq!(flags.is_always_start_anchored, (word / 4) % 2 != 0);
                assert_eq!(consumed, 4);
                true
            }
        }
    }
    assert_eq!(usize::BITS, 64);
    let mut counts = [0; 3];
    let upper = [0, 0xFFFF_FFF8, 0xAAAA_AAA8, 0x5555_5550];
    for high in upper.into_iter().chain((3..32).map(|bit| 1u32 << bit)) {
        for low in 0..8 {
            let word = high + low;
            for offset in 0..8 {
                for len in 0..=12 {
                    for tail in [0, 0xA5, 0xFF] {
                        let mut backing = [tail; 24];
                        backing[offset..offset + 4].copy_from_slice(&prefix(word));
                        let original = backing;
                        let success = check(&backing[offset..offset + len], word);
                        assert_eq!(backing, original, "input was changed");
                        counts[usize::from(success)] += 1;
                        counts[2] += 1;
                    }
                }
            }
        }
    }
    assert_eq!(counts, [25_344, 57_024, 82_368]);
    counts
}
'''

WIRE = r'''
#[doc(hidden)]
pub(crate) fn __rstyle_wire_flag_parser_error(error: DeserializeError) {
    match error.0 {
        DeserializeErrorKind::BufferTooSmall { what } => assert_eq!(what, "flag bitset"),
        _ => panic!("wrong original short-input error variant"),
    }
}
'''

BRIDGE = r'''
#[doc(hidden)]
pub fn __rstyle_wire_flag_parser_oracle() -> [u64; 3] {
    dfa::dense::__rstyle_wire_flag_parser_oracle()
}
'''

RUNNER = r'''
fn main() {
    let counts = regex_automata::__rstyle_wire_flag_parser_oracle();
    println!("{{\"short_errors\":{},\"successes\":{},\"total\":{},\"debug_assertions\":{},\"mismatches\":0}}",
        counts[0], counts[1], counts[2], cfg!(debug_assertions));
}
'''


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def verify(selection, root, common):
    common.verify_sources(selection, root)
    declaration = next(item["source"] for item in selection["source_obligations"]
                       if item["name"] == "complete DeserializeErrorKind declaration")
    if re.findall(r"^    ([A-Za-z]+) \{", declaration, re.MULTILINE) != selection["error_variant_roster"]:
        raise ValueError("Complete error variant roster differs")
    if selection["functions"][0]["name"] != "dfa::dense::Flags::from_bytes":
        raise ValueError("Target differs from the source-selected original parser")
    expected_corpus = dict(low_bit_combinations=8, upper_bit_patterns=33,
                           offsets=list(range(8)), lengths=list(range(13)), tail_bytes=[0, 165, 255])
    if selection["corpus"] != expected_corpus:
        raise ValueError("Independent corpus specification differs")
    factor = 8 * 33 * 8 * 3
    if selection["expected_cases_per_context"] != dict(short_errors=factor * 4,
                                                       successes=factor * 9, total=factor * 13):
        raise ValueError("Frozen independent case inventory differs")
    active, inactive = selection["build_selection"]["contexts"]
    if (active["name"], active["default_features"], active["features"], active["expected_activity"]) != (
            "default", True, [], "active") or "dfa-search" not in active["expected_feature_closure"]:
        raise ValueError("Active context must preserve the original default build")
    if (inactive["name"], inactive["default_features"], inactive["features"], inactive["expected_activity"]) != (
            "no-default", False, [], "inactive: dfa-search module unavailable"):
        raise ValueError("No-default must remain inactive")


def appendages():
    return {Path(path): b"\n\n" + value.encode() for path, value in [
        ("src/dfa/dense.rs", DENSE), ("src/util/wire.rs", WIRE), ("src/lib.rs", BRIDGE)]}


def environment(selection, args):
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("RSTYLE_", "CARGO_PROFILE_", "CARGO_TARGET_"))
           and key not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                           "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET"}}
    env["RUSTUP_TOOLCHAIN"] = selection["build_selection"]["toolchain"]
    version = subprocess.check_output([args.rustc, "-vV"], text=True, env=env)
    compiler = dict(line.split(": ", 1) for line in version.splitlines() if ": " in line)
    if compiler.get("commit-hash") != selection["build_selection"]["rustc_commit"]:
        raise ValueError("Native compiler differs from selected compiler")
    if compiler.get("host") not in selection["build_selection"]["targets"]:
        raise ValueError("Native host outside selected 64-bit hosts")
    return env, compiler["host"]


def native(selection, root, args, here, common):
    env, host = environment(selection, args)
    copy_checks = module("original_copy_checks", here.parent / "time-digit-prefixes/oracle.py")
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-wire-flag-parser-") as temporary:
        copied = Path(temporary) / "original"
        shutil.copytree(root, copied, ignore=shutil.ignore_patterns("target", ".git"))
        for path in copied.rglob("*"):
            if path.is_file():
                path.chmod(path.stat().st_mode | 0o200)
        additions = appendages()
        for path, extra in additions.items():
            (copied / path).write_bytes((root / path).read_bytes() + extra)
        copy_checks.verify_copy(root, copied, additions)
        runner = Path(temporary) / "runner"
        (runner / "src").mkdir(parents=True)
        (runner / "src/main.rs").write_text(RUNNER)
        modes = [True, False] if args.debug_mode == "both" else [args.debug_mode == "debug"]
        for debug in modes:
            (runner / "Cargo.toml").write_text(
                '[package]\nname="original_wire_flag_parser_oracle"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
                '[dependencies]\nregex-automata={path="../original",default-features=true}\n'
                '[profile.dev]\nopt-level=0\noverflow-checks=true\npanic="unwind"\ndebug-assertions='
                + str(debug).lower() + '\n')
            command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                       str(runner / "Cargo.toml"), "--target-dir", str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=env, check=True, text=True,
                                       capture_output=True, timeout=args.timeout)
            record = json.loads(completed.stdout)
            if (any(record.get(key) != count for key, count in selection["expected_cases_per_context"].items())
                    or record.get("mismatches") != 0 or record.get("debug_assertions") != debug):
                raise ValueError(f"Oracle case inventory or context differs: {record}")
            records.append(dict(record, context="default", host=host, admission_evaluated=False))
        copy_checks.verify_copy(root, copied, additions)
    verify(selection, root, common)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="coordinator schedules Cargo runs")
    parser.add_argument("--debug-mode", choices=["debug", "release-branch", "both"], default="both")
    parser.add_argument("--target-dir", type=Path, default=Path("target/wire-flag-parser-oracle"))
    parser.add_argument("--cargo", default=str(Path.home() / ".cargo/bin/cargo"))
    parser.add_argument("--rustc", default=str(Path.home() / ".cargo/bin/rustc"))
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    selection = json.loads((here / "selection.json").read_text())
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    common = module("source_checks", here.parent / "checked-index-decode/oracle.py")
    verify(selection, root, common)
    result = dict(identity="rstyle.original-wire-flag-parser-oracle/1", source_identities_checked=True,
                  checksum="XXH3-64", admission_evaluated=False, contexts=[],
                  inactive_contexts=[dict(context="no-default", reason="dfa-search module unavailable")])
    if args.run_native:
        result["contexts"] = native(selection, root, args, here, common)
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
