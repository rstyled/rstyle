#!/usr/bin/env python3
"""Source-first widening-reader selection and append-only original-crate oracle."""
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
    expected = {"util::wire::try_read_u32_as_usize", "util::wire::try_read_u16_as_usize"}
    if {row["name"] for row in selection["functions"] if row["role"] == "target"} != expected:
        raise ValueError("Frozen widening-reader targets differ")
    declaration = next(row["source"] for row in selection["source_obligations"]
                       if row["name"] == "complete DeserializeErrorKind declaration")
    if re.findall(r"^    ([A-Za-z]+) \{", declaration, re.MULTILINE) != selection["error_variant_roster"]:
        raise ValueError("Complete error declaration roster differs")
    corpus = selection["corpus"]
    if (corpus != dict(u16_exhaustive=True, u16_boundaries=[0, 1, 255, 256, 65534, 65535],
                       u32_boundaries=[0, 1, 255, 256, 65535, 65536, 4294967294, 4294967295],
                       one_hot_bits={"u16": list(range(16)), "u32": list(range(32))},
                       samples=128, seed=2654435769, multiplier=1664525, increment=1013904223,
                       offsets=list(range(4)), full_extra_lengths=list(range(5)),
                       short_patterns=[0, 127, 128, 255], tail_bytes=[0, 165, 255],
                       messages=["", "distinct static LEFT", "distinct static RIGHT"])):
        raise ValueError("Independent corpus changed")
    counts = dict(exhaustive_u16=65536 * 2,
                  full_inputs=((6 + 16 + 128) + (8 + 32 + 128)) * 4 * 5 * 3 * 3 * 2,
                  short_errors=(2 + 4) * 4 * 4 * 3 * 2, constructor_controls=3 * 2)
    counts["total"] = sum(counts.values())
    if selection["expected_cases_per_context"] != counts:
        raise ValueError("Independent case denominators changed")
    if [(row["name"], row["default_features"], row["features"])
            for row in selection["build_selection"]["contexts"]] != [
                ("default", True, []), ("no-default", False, [])]:
        raise ValueError("Original build selections changed")


def native(selection, root, args, here, common):
    controls = module("native_controls", here.parent / "wire-flag-parser/oracle.py")
    copies = module("whole_copy_checks", here.parent / "time-digit-prefixes/oracle.py")
    env, host = controls.environment(selection, args)
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-wire-usize-readers-") as temporary:
        copied = Path(temporary) / "original"
        shutil.copytree(root, copied, ignore=shutil.ignore_patterns("target", ".git"))
        for path in copied.rglob("*"):
            if path.is_file():
                path.chmod(path.stat().st_mode | 0o200)
        additions = {
            Path("src/util/wire.rs"): b"\n\n" + (here / "oracle.rs").read_bytes(),
            Path("src/lib.rs"): b'''\n\n#[doc(hidden)]
pub fn __rstyle_usize_reader_probe(api: u8, slice: &[u8], what: &'static str)
    -> (u8, u64, usize, &'static str) {
    util::wire::__rstyle_usize_reader_probe(api, slice, what)
}
''',
        }
        for path, extra in additions.items():
            (copied / path).write_bytes((root / path).read_bytes() + extra)
        copies.verify_copy(root, copied, additions)
        runner = Path(temporary) / "runner"
        (runner / "src").mkdir(parents=True)
        (runner / "src/main.rs").write_bytes((here / "runner.rs").read_bytes())
        modes = [True, False] if args.debug_mode == "both" else [args.debug_mode == "debug"]
        for context in selection["build_selection"]["contexts"]:
            if args.context not in ("both", context["name"]):
                continue
            for debug in modes:
                (runner / "Cargo.toml").write_text(
                    '[package]\nname="original_wire_usize_readers_oracle"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
                    '[dependencies]\nregex-automata={path="../original",default-features='
                    + str(context["default_features"]).lower() + '}\n'
                    '[profile.dev]\nopt-level=0\noverflow-checks=true\npanic="unwind"\ndebug-assertions='
                    + str(debug).lower() + '\n')
                command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                           str(runner / "Cargo.toml"), "--target-dir", str(args.target_dir.resolve())]
                completed = subprocess.run(command, env=env, text=True, capture_output=True, timeout=args.timeout)
                if completed.returncode:
                    raise RuntimeError(f"Original oracle failed context={context['name']} debug={debug}:\n{completed.stderr}")
                record = json.loads(completed.stdout)
                if (any(record.get(key) != value for key, value in selection["expected_cases_per_context"].items())
                        or record.get("mismatches") != 0 or record.get("debug_assertions") != debug):
                    raise ValueError(f"Independent oracle inventory/context differs: {record}")
                records.append(dict(record, context=context["name"], host=host, admission_evaluated=False))
        copies.verify_copy(root, copied, additions)
    verify(selection, root, common)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true")
    parser.add_argument("--context", choices=["default", "no-default", "both"], default="both")
    parser.add_argument("--debug-mode", choices=["debug", "release-branch", "both"], default="both")
    parser.add_argument("--target-dir", type=Path, default=Path("target/wire-usize-readers-oracle"))
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
    result = dict(identity="rstyle.original-wire-usize-readers-oracle/1", source_identities_checked=True,
                  checksum="XXH3-64", admission_evaluated=False, contexts=[])
    if args.run_native:
        result["contexts"] = native(selection, root, args, here, common)
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
