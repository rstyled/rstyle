#!/usr/bin/env python3
"""Source-first wire arithmetic audit; opt-in append-only original-crate oracle."""
import argparse
import importlib.util
import json
import os
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
    declaration = next(item["source"] for item in selection["source_obligations"]
                       if item["name"] == "complete DeserializeErrorKind declaration")
    roster = re.findall(r"^    ([A-Za-z]+) \{", declaration, re.MULTILINE)
    if roster != selection["error_variant_roster"]:
        raise ValueError("Complete ten-variant error inventory changed")


def appendages(here):
    return {
        Path("src/util/wire.rs"): b"\n\n" + (here / "oracle.rs").read_bytes(),
        Path("src/lib.rs"): b'''\n\n#[doc(hidden)]
pub fn __rstyle_wire_arithmetic_oracle() -> (u64, u64, u64) {
    util::wire::__rstyle_wire_arithmetic_oracle()
}
''',
    }


def native(selection, root, args, here, common):
    toolchain = selection["build_selection"]["toolchain"]
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("RSTYLE_", "CARGO_PROFILE_", "CARGO_TARGET_"))
           and key not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                           "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET"}}
    env["RUSTUP_TOOLCHAIN"] = toolchain
    version = subprocess.check_output([args.rustc, "-vV"], text=True, env=env)
    compiler = dict(line.split(": ", 1) for line in version.splitlines() if ": " in line)
    if compiler.get("commit-hash") != selection["build_selection"]["rustc_commit"]:
        raise ValueError("Native compiler differs from frozen selection")
    if compiler.get("host") not in selection["build_selection"]["targets"]:
        raise ValueError("Native host outside selected 64-bit hosts")
    copy_checks = module("original_copy_checks", here.parent / "time-digit-prefixes/oracle.py")
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-wire-arithmetic-") as temporary:
        copied = Path(temporary) / "original"
        shutil.copytree(root, copied, ignore=shutil.ignore_patterns("target", ".git"))
        for path in copied.rglob("*"):
            if path.is_file():
                path.chmod(path.stat().st_mode | 0o200)
        additions = appendages(here)
        for path, extra in additions.items():
            (copied / path).write_bytes((root / path).read_bytes() + extra)
        copy_checks.verify_copy(root, copied, additions)
        runner = Path(temporary) / "runner"
        (runner / "src").mkdir(parents=True)
        (runner / "src/main.rs").write_text('''fn main() {
    let (checks, success, overflow) = regex_automata::__rstyle_wire_arithmetic_oracle();
    println!("{{\\\"total_checks\\\":{},\\\"success\\\":{},\\\"overflow\\\":{},\\\"mismatches\\\":0}}", checks, success, overflow);
}
''')
        modes = [True, False] if args.debug_mode == "both" else [args.debug_mode == "debug"]
        for context in selection["build_selection"]["contexts"]:
            if args.context not in ("both", context["name"]):
                continue
            for debug in modes:
                (runner / "Cargo.toml").write_text(
                    '[package]\nname="original_wire_arithmetic_oracle"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
                    '[dependencies]\nregex-automata={path="../original",default-features='
                    + str(context["default_features"]).lower() + '}\n'
                    '[profile.dev]\nopt-level=0\noverflow-checks=true\npanic="unwind"\ndebug-assertions='
                    + str(debug).lower() + '\n')
                command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                           str(runner / "Cargo.toml"), "--target-dir", str(args.target_dir.resolve())]
                completed = subprocess.run(command, env=env, check=True, text=True,
                                           capture_output=True, timeout=args.timeout)
                record = json.loads(completed.stdout)
                if record["total_checks"] != selection["expected_checks_per_context"] or record["mismatches"] != 0:
                    raise ValueError(f"Oracle case inventory differs: {record}")
                records.append(dict(record, context=context["name"], debug_assertions=debug,
                                    host=compiler["host"], admission_evaluated=False))
        copy_checks.verify_copy(root, copied, additions)
    verify(selection, root, common)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="coordinator schedules Cargo runs")
    parser.add_argument("--context", choices=["default", "no-default", "both"], default="both")
    parser.add_argument("--debug-mode", choices=["debug", "release-branch", "both"], default="both")
    parser.add_argument("--target-dir", type=Path, default=Path("target/wire-arithmetic-oracle"))
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
    result = dict(identity="rstyle.original-wire-arithmetic-oracle/1", source_identities_checked=True,
                  error_variants_checked=10, checksum="XXH3-64", admission_evaluated=False, contexts=[])
    if args.run_native:
        result["contexts"] = native(selection, root, args, here, common)
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
