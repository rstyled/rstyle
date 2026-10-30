#!/usr/bin/env python3
"""Check original LookSet::available exhaustively over low bits in native contexts.

This deliberately uses the complete original dependency. It neither copies the
private helper into the oracle nor derives an expectation from extractor output.
Cross-target capture/strict admission remains a separate nine-context gate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def verify_sources(selection, package):
    for relative, expected in selection["package"]["file_sha256"].items():
        observed = hashlib.sha256((package / relative).read_bytes()).hexdigest()
        if observed != expected:
            raise RuntimeError(f"Original package source changed: {relative}")
    for function in selection["functions"] + selection["owners"]:
        source = (package / function["file"]).read_bytes()
        span = function["span"]
        original = source[span["start"]:span["end"]]
        if original.decode() != function["source"]:
            raise RuntimeError(f"Original function span changed: {function['name']}")
        if hashlib.sha256(original).hexdigest() != function["sha256"]:
            raise RuntimeError(f"Original function identity changed: {function['name']}")


def manifest(package, context):
    arguments = context["cargo_arguments"]
    enabled = "--no-default-features" not in arguments
    features = []
    if "--features" in arguments:
        features = arguments[arguments.index("--features") + 1].split(",")
    return (
        '[package]\nname="original_result_propagation"\nversion="0.0.0"\n'
        'edition="2021"\n[workspace]\n[dependencies.regex-automata]\n'
        f'path={json.dumps(str(package))}\n'
        f'default-features={str(enabled).lower()}\n'
        f'features={json.dumps(features)}\n'
    )


RUNNER = """use regex_automata::util::look::LookSet;

fn check(bits: u32, enabled: bool) {
    let expected_error = !enabled && bits & 0x33300 != 0;
    let observed = LookSet { bits }.available();
    assert_eq!(observed.is_err(), expected_error, "bits={bits:#010x}");
    if let Err(error) = observed {
        // Preserve the original nominal, including its payload and destructor.
        assert_eq!(std::mem::size_of_val(&error), 0);
    }
}

fn main() {
    let enabled = std::env::args().nth(1).unwrap() == "enabled";
    let mut cases = 0u64;
    // Numeric expectations are independent of the extractor and API flag helpers.
    for high in [0, 1 << 18, 1 << 31, 0xfffc0000, 0x55540000, 0xaaa80000] {
        for low in 0..(1u32 << 18) {
            check(low | high, enabled);
            cases += 1;
        }
    }
    for bit in 18..32 {
        for low in [0, 1, 1 << 6, 1 << 8, 1 << 9, 1 << 12, 1 << 13,
                    1 << 16, 1 << 17, 0x33300, 0xcccff, 0x3ffff] {
            check(low | (1 << bit), enabled);
            cases += 1;
        }
    }
    println!("{cases}");
}
"""


def run(args, selection, package):
    records = []
    environment = dict(os.environ)
    environment["RUSTUP_TOOLCHAIN"] = selection["build_selection"]["toolchain"]
    with tempfile.TemporaryDirectory(prefix="rstyle-result-propagation-") as temporary:
        workspace = Path(temporary)
        (workspace / "src").mkdir()
        (workspace / "src/main.rs").write_text(RUNNER)
        for context in selection["build_selection"]["contexts"]:
            (workspace / "Cargo.toml").write_text(manifest(package, context))
            command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                       str(workspace / "Cargo.toml")]
            if args.target_dir:
                command += ["--target-dir", str(args.target_dir.resolve())]
            enabled = context["expected_active_branch"] != "unavailable"
            command += ["--", "enabled" if enabled else "disabled"]
            completed = subprocess.run(command, env=environment, text=True,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            if completed.returncode:
                raise RuntimeError(completed.stderr)
            cases = int(completed.stdout.strip())
            expected_cases = 6 * (1 << 18) + 14 * 12
            if cases != expected_cases:
                raise RuntimeError(f"Incomplete oracle: {cases} != {expected_cases}")
            records.append({"context": context["expected_active_branch"],
                            "unicode_word_boundary": enabled,
                            "cases": cases, "mismatches": 0})
    return {"identity": "rstyle.original-result-propagation-oracle/1",
            "scope": "unchanged full-library public API; native execution only",
            "source_hashes_checked": True, "contexts": records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-root", type=Path)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    selection_path = Path(__file__).resolve().parent.parent / "result-propagation-selection.json"
    selection = json.loads(selection_path.read_text())
    package = (args.package_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, package)
    result = json.dumps(run(args, selection, package), indent=2) + "\n"
    if args.output:
        args.output.write_text(result)
    print(result, end="")


if __name__ == "__main__":
    main()
