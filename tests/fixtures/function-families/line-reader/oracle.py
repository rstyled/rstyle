#!/usr/bin/env python3
"""Verify original bytes; optionally execute an assembled private-source witness.

The original LineReader owner/method are included verbatim, with an adapted
public Error import. This is separate from unchanged whole-crate admission.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


RUNNER = r'''
fn check(result: Result<LineReader<'_>, Error>, input: &[u8], counts: &mut [u64; 2]) {
    if input.len() == 0 {
        match result {
            Err(error) => assert_eq!(error, Error::InvalidLength),
            Ok(_) => panic!("empty input must fail"),
        }
        counts[1] += 1;
    } else {
        let reader = match result {
            Ok(reader) => reader,
            Err(error) => panic!("nonempty input failed: {error:?}"),
        };
        assert_eq!(reader.remaining.len(), input.len());
        assert_eq!(reader.remaining, input);
        assert_eq!(reader.remaining.as_ptr(), input.as_ptr(), "wrong backing origin");
        assert_eq!(reader.line_width, None);
        counts[0] += 1;
    }
}

fn main() {
    let mut counts = [0u64; 2];
    for pattern in 0..3 {
        let backing: Vec<u8> = (0..520).map(|index| match pattern {
            0 => index as u8,
            1 => 255 - index as u8,
            _ => 0xa5,
        }).collect();
        for offset in [0, 1, 7] {
            for length in 0..=512 {
                let input = &backing[offset..offset + length];
                check(LineReader::new_unwrapped(input), input, &mut counts);
                // The stored reference belongs to the backing, independent of
                // this local reference variable's shorter scope.
                let result = {
                    let temporary_reference = &backing[offset..offset + length];
                    LineReader::new_unwrapped(temporary_reference)
                };
                check(result, input, &mut counts);
            }
        }
    }
    println!("{{\"pointer_width\":{},\"successes\":{},\"errors\":{}}}",
             usize::BITS, counts[0], counts[1]);
}
'''


def verify_sources(selection, package):
    """Check frozen bytes/ranges and reused identity references, with no hashing."""
    items = selection["functions"] + selection["owners"] + selection["source_obligations"]
    for item in items:
        raw = (package / item["file"]).read_bytes()
        span = item["span"]
        if raw[span["start"]:span["end"]] != item["source"].encode():
            raise RuntimeError(f"Original source/range changed: {item['name']}")
        if item.get("complete_file") and raw != item["source"].encode():
            raise RuntimeError(f"Complete original file changed: {item['file']}")
    identity = selection["package"]["historical_source_identity"]
    historical_path = Path(__file__).resolve().parent / identity["selection"]
    historical = json.loads(historical_path.read_text())["package"]
    if historical["file_sha256"] != identity["files"]:
        raise RuntimeError("Historical source identity references changed")
    for key in ["name", "version"]:
        if historical[key] != selection["package"][key]:
            raise RuntimeError(f"Historical package identity changed: {key}")


def assembled_source(selection):
    return ("use base64ct::Error::{self, InvalidLength};\n\n"
            + selection["owners"][0]["source"]
            + "\n\nimpl<'i> LineReader<'i> {\n"
            + selection["functions"][0]["source"] + "\n}\n" + RUNNER)


def manifest(package, context):
    return ('[package]\nname="original_line_reader_witness"\nversion="0.0.0"\n'
            'edition="2021"\n[workspace]\n[dependencies.base64ct]\n'
            f'path={json.dumps(str(package))}\n'
            'default-features=false\n'
            f'features={json.dumps(context["features"])}\n'
            '[profile.dev]\nopt-level=0\noverflow-checks=true\n'
            'debug-assertions=true\npanic="unwind"\n')


def native(args, selection, package):
    environment = dict(os.environ)
    environment["RUSTUP_TOOLCHAIN"] = selection["build_selection"]["toolchain"]
    for key in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                "RUSTC_WORKSPACE_WRAPPER", "RUSTC", "CARGO_BUILD_TARGET"]:
        environment.pop(key, None)
    for key in list(environment):
        if key.startswith("CARGO_PROFILE_DEV_"):
            environment.pop(key)
    compiler = subprocess.run(["rustc", "-vV"], env=environment, text=True,
                              capture_output=True, check=True, timeout=30).stdout
    metadata = dict(line.split(": ", 1) for line in compiler.splitlines() if ": " in line)
    if metadata.get("commit-hash") != selection["build_selection"]["rustc_commit"]:
        raise RuntimeError("Compiler differs from frozen selection")
    if metadata.get("host") not in selection["build_selection"]["targets"]:
        raise RuntimeError("Native host is outside the frozen target selection")
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-line-reader-witness-") as temporary:
        workspace = Path(temporary)
        (workspace / "src").mkdir()
        (workspace / "src/main.rs").write_text(assembled_source(selection))
        for context in selection["build_selection"]["contexts"]:
            (workspace / "Cargo.toml").write_text(manifest(package, context))
            command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                       str(workspace / "Cargo.toml"), "--target", metadata["host"]]
            if args.target_dir:
                command += ["--target-dir", str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=environment, cwd=workspace,
                                       text=True, capture_output=True, timeout=args.timeout)
            if completed.returncode:
                raise RuntimeError(completed.stderr)
            result = json.loads(completed.stdout)
            if result != {"pointer_width": 64, "successes": 9216, "errors": 18}:
                raise RuntimeError(f"Unexpected or incomplete oracle counts: {result}")
            records.append({"context": context["name"], **result, "mismatches": 0})
    return {"native_host": metadata["host"], "rustc": metadata, "contexts": records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-root", type=Path)
    parser.add_argument("--run-native", action="store_true",
                        help="build/run the witness only after coordinator gates finish")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    selection = json.loads(Path(__file__).with_name("selection.json").read_text())
    package = (args.package_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, package)
    result = {"identity": "rstyle.original-line-reader-witness/1",
              "scope": "assembled private exact-source witness; no whole-crate admission claim",
              "selected_source_ranges_checked": True,
              "historical_identity_reference_checked": True,
              "whole_package_source_identity_recomputed": False,
              "execution": "native witness" if args.run_native else "not performed"}
    if args.run_native:
        result.update(native(args, selection, package))
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
