#!/usr/bin/env python3
"""Run the unchanged public Result-returning API in three native contexts.

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
    for function in selection["functions"]:
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
        '[package]\nname="original_result_check"\nversion="0.0.0"\n'
        'edition="2021"\n[workspace]\n[dependencies.regex-automata]\n'
        f'path={json.dumps(str(package))}\n'
        f'default-features={str(enabled).lower()}\n'
        f'features={json.dumps(features)}\n'
    )


RUNNER = """fn main() {
    match regex_automata::util::look::UnicodeWordBoundaryError::check() {
        Ok(()) => println!("ok"),
        Err(error) => {
            // Preserve the actual nominal error value; never reinterpret it as ().
            assert_eq!(std::mem::size_of_val(&error), 0);
            println!("err");
        }
    }
}
"""


def run(args, selection, package):
    records = []
    environment = dict(os.environ)
    environment["RUSTUP_TOOLCHAIN"] = selection["build_selection"]["toolchain"]
    with tempfile.TemporaryDirectory(prefix="rstyle-original-result-") as temporary:
        workspace = Path(temporary)
        (workspace / "src").mkdir()
        (workspace / "src/main.rs").write_text(RUNNER)
        for context in selection["build_selection"]["contexts"]:
            (workspace / "Cargo.toml").write_text(manifest(package, context))
            command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                       str(workspace / "Cargo.toml")]
            if args.target_dir:
                command += ["--target-dir", str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=environment, text=True,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            if completed.returncode:
                raise RuntimeError(completed.stderr)
            observed = completed.stdout.strip()
            expected = "err" if context["expected_active_branch"] == "unavailable" else "ok"
            if observed != expected:
                raise RuntimeError(f"{context['expected_active_branch']}: {observed} != {expected}")
            records.append({"context": context["expected_active_branch"],
                            "expected": expected, "observed": observed})
    return {"identity": "rstyle.original-result-oracle/1",
            "scope": "unchanged full-library public API; native execution only",
            "source_hashes_checked": True, "contexts": records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-root", type=Path)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    selection_path = Path(__file__).resolve().parent.parent / "scalar-result-next-selection.json"
    selection = json.loads(selection_path.read_text())
    package = (args.package_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, package)
    result = json.dumps(run(args, selection, package), indent=2) + "\n"
    if args.output:
        args.output.write_text(result)
    print(result, end="")


if __name__ == "__main__":
    main()
