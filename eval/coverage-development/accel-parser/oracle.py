#!/usr/bin/env python3
"""Frozen original Accel parser contract; native execution is opt-in."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def verify(selection, root, common):
    common.verify_sources(selection, root)
    targets = {row["name"] for row in selection["functions"] if row["role"].startswith("target:")}
    if targets != {"dfa::accel::Accel::" + name for name in
                   ["from_slice", "from_bytes", "from_bytes_unchecked"]}:
        raise ValueError("Complete original target family changed")
    obligations = {row["name"]: row["source"] for row in selection["source_obligations"]}
    if re.findall(r"^    ([A-Za-z]+) \{", obligations["complete DeserializeErrorKind declaration"],
                  re.MULTILINE) != selection["error_variant_roster"]:
        raise ValueError("Complete error roster changed")
    if not obligations["complete derived Accel declaration"].startswith("#[derive(Clone)]"):
        raise ValueError("Original non-Copy owner changed")
    expected_corpus = dict(first_bytes=list(range(256)), payload_patterns=[0, 165, 255],
                           offsets=list(range(4)), lengths=list(range(9)), tail_bytes=[0, 165, 255],
                           payload_positions=[1, 2, 3], payload_values=list(range(256)),
                           payload_first_bytes=[0, 1, 2, 3], payload_lengths=[4, 8],
                           direct_apis=["from_bytes", "from_bytes_unchecked"])
    if selection["corpus"] != expected_corpus:
        raise ValueError("Independent input corpus changed")
    counts = dict(short_errors=256 * 3 * 4 * 4 * 3,
                  slice_invalid_errors=252 * 3 * 4 * 5 * 3,
                  slice_successes=4 * 3 * 4 * 5 * 3 + 3 * 256 * 4 * 4 * 2 * 3,
                  array_invalid_errors=252 * 3,
                  array_successes=4 * 3 + 3 * 256 * 4,
                  unchecked_controls=256 * 3 + 3 * 256 * 4)
    counts["total"] = sum(counts.values())
    if selection["expected_cases_per_context"] != counts:
        raise ValueError("Independent case denominators changed")
    contexts = [(c["name"], c["default_features"], c["features"], c["expected_activity"])
                for c in selection["build_selection"]["contexts"]]
    if contexts != [("default", True, [], "active"),
                    ("dfa-search-only", False, ["dfa-search"], "active"),
                    ("no-default", False, [], "inactive: dfa-search module unavailable")]:
        raise ValueError("Source-selected Cargo contexts changed")


def appendages(here):
    wire = b'''
#[doc(hidden)]
pub(crate) fn __rstyle_accel_parser_error(error: DeserializeError) -> (u8, &'static str) {
    match error.0 {
        DeserializeErrorKind::BufferTooSmall { what } => (1, what),
        DeserializeErrorKind::Generic { msg } => (2, msg),
        _ => panic!("unexpected original error variant"),
    }
}
'''
    bridge = b'''
#[doc(hidden)]
pub fn __rstyle_accel_parser_probe(api: u8, input: &[u8]) -> (u8, [u8; 8], &'static str) {
    dfa::accel::__rstyle_accel_parser_probe(api, input)
}
'''
    return {Path("src/dfa/accel.rs"): b"\n\n" + (here / "oracle.rs").read_bytes(),
            Path("src/util/wire.rs"): b"\n\n" + wire,
            Path("src/lib.rs"): b"\n\n" + bridge}


def native(selection, root, args, here, common):
    controls = module("native_controls", here.parent / "wire-flag-parser/oracle.py")
    copies = module("whole_copy_checks", here.parent / "time-digit-prefixes/oracle.py")
    env, host = controls.environment(selection, args)
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-accel-parser-") as temporary:
        copied = Path(temporary) / "original"
        shutil.copytree(root, copied, ignore=shutil.ignore_patterns("target", ".git"))
        for path in copied.rglob("*"):
            if path.is_file():
                path.chmod(path.stat().st_mode | 0o200)
        additions = appendages(here)
        for path, extra in additions.items():
            (copied / path).write_bytes((root / path).read_bytes() + extra)
        copies.verify_copy(root, copied, additions)
        runner = Path(temporary) / "runner"
        (runner / "src").mkdir(parents=True)
        (runner / "src/main.rs").write_bytes((here / "runner.rs").read_bytes())
        modes = [True, False] if args.debug_mode == "both" else [args.debug_mode == "debug"]
        for context in selection["build_selection"]["contexts"]:
            if context["expected_activity"] != "active" or args.context not in ("both", context["name"]):
                continue
            for debug in modes:
                manifest = ('[package]\nname="original_accel_parser_oracle"\nversion="0.0.0"\n'
                            'edition="2021"\n[workspace]\n[dependencies]\n'
                            'regex-automata={path="../original",default-features='
                            + str(context["default_features"]).lower() + ',features='
                            + json.dumps(context["features"]) + '}\n'
                            '[profile.dev]\nopt-level=0\noverflow-checks=true\npanic="unwind"\n'
                            'debug-assertions=' + str(debug).lower() + '\n')
                (runner / "Cargo.toml").write_text(manifest)
                command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                           str(runner / "Cargo.toml"), "--target-dir", str(args.target_dir.resolve())]
                completed = subprocess.run(command, env=env, text=True, capture_output=True,
                                           timeout=args.timeout)
                if completed.returncode:
                    raise RuntimeError(f"Original oracle failed {context['name']} debug={debug}:\n{completed.stderr}")
                record = json.loads(completed.stdout)
                if any(record.get(k) != v for k, v in selection["expected_cases_per_context"].items()) or (
                        record.get("debug_assertions") != debug or record.get("mismatches") != 0):
                    raise ValueError(f"Independent inventory/context changed: {record}")
                records.append(dict(record, context=context["name"], host=host, admission_evaluated=False))
        copies.verify_copy(root, copied, additions)
    verify(selection, root, common)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="coordinator schedules Cargo runs")
    parser.add_argument("--context", choices=["default", "dfa-search-only", "both"], default="both")
    parser.add_argument("--debug-mode", choices=["debug", "release-branch", "both"], default="both")
    parser.add_argument("--target-dir", type=Path, default=Path("target/accel-parser-oracle"))
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
    result = dict(identity="rstyle.original-accel-parser-oracle/1", source_identities_checked=True,
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
