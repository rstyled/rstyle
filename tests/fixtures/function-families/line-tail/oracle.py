#!/usr/bin/env python3
"""Verify frozen original bytes; optionally run a private exact-source witness.

This assembles the unchanged Line declaration and two private methods. It links
the full original base64ct package for Error, but it is not whole-crate admission
evidence or an unchanged-public-API test of the private methods.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


RUNNER = r'''
fn check(line: &Line<'_>, nbytes: usize, counts: &mut [u64; 2]) {
    let remaining = line.remaining;
    assert_eq!(line.len(), remaining.len());
    let observed = line.slice_tail(nbytes);
    if nbytes > remaining.len() {
        assert_eq!(observed, Err(Error::InvalidLength));
        counts[1] += 1;
    } else {
        let actual = observed.expect("a valid tail must succeed");
        let expected = &remaining[remaining.len() - nbytes..];
        assert_eq!(actual.len(), nbytes);
        assert_eq!(actual, expected);
        assert_eq!(actual.as_ptr(), expected.as_ptr(), "wrong backing origin");
        counts[0] += 1;
    }
    assert_eq!(line.len(), remaining.len());
    assert_eq!(line.remaining.as_ptr(), remaining.as_ptr());
    assert_eq!(line.remaining.len(), remaining.len());
}

fn main() {
    let mut counts = [0u64; 2];
    let mut lifetime_queries = 0u64;
    for pattern in 0..3 {
        let backing: Vec<u8> = (0..520).map(|index| match pattern {
            0 => index as u8,
            1 => 255 - index as u8,
            _ => 0xa5,
        }).collect();
        for offset in [0, 1, 7] {
            for length in 0..=512 {
                let remaining = &backing[offset..offset + length];
                let line = Line { remaining };
                for nbytes in 0..=length {
                    check(&line, nbytes, &mut counts);
                }
                for nbytes in [length + 1, usize::MAX / 2,
                               usize::MAX - 1, usize::MAX] {
                    check(&line, nbytes, &mut counts);
                }
                // The result belongs to the backing slice, beyond the lifetime
                // of the temporary Line value and its shared receiver borrow.
                let tail = {
                    let temporary_line = Line { remaining };
                    temporary_line.slice_tail(length / 2).unwrap()
                };
                let expected = &remaining[length - length / 2..];
                assert_eq!(tail, expected);
                assert_eq!(tail.as_ptr(), expected.as_ptr());
                lifetime_queries += 1;
            }
        }
    }
    println!("{{\"pointer_width\":{},\"valid_queries\":{},\"invalid_queries\":{},\"lifetime_queries\":{}}}",
             usize::BITS, counts[0], counts[1], lifetime_queries);
}
'''


def verify_sources(selection, package):
    """Compare exact frozen bytes; do not mint/recompute source digests."""
    for item in selection["functions"] + selection["source_obligations"]:
        raw = (package / item["file"]).read_bytes()
        span = item["span"]
        if raw[span["start"]:span["end"]] != item["source"].encode():
            raise RuntimeError(f"Original source/range changed: {item['name']}")
        if item.get("complete_file") and raw != item["source"].encode():
            raise RuntimeError(f"Original complete file changed: {item['file']}")
    historical_path = Path(__file__).resolve().parent / selection["package"][
        "historical_source_identity"]["selection"]
    historical = json.loads(historical_path.read_text())
    identity = selection["package"]["historical_source_identity"]
    if historical["package"]["file_sha256"] != identity["files"]:
        raise RuntimeError("Referenced historical source identities changed")
    for key in ["name", "version"]:
        if historical["package"][key] != selection["package"][key]:
            raise RuntimeError(f"Historical package identity differs: {key}")


def assembled_source(selection):
    owner = next(item for item in selection["source_obligations"]
                 if item["name"] == "decoder::Line<'i>")
    methods = "\n\n".join(item["source"] for item in selection["functions"])
    # Only the public dependency path is adapted; method/declaration bytes are
    # included verbatim. Original lexical imports remain frozen obligations.
    return ("use base64ct::Error::{self, InvalidLength};\n\n"
            + owner["source"] + "\n\nimpl<'i> Line<'i> {\n"
            + methods + "\n}\n" + RUNNER)


def manifest(package, context):
    return ('[package]\nname="original_line_tail_witness"\nversion="0.0.0"\n'
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
    compiler = subprocess.run(["rustc", "-vV"], env=environment,
                              text=True, capture_output=True, check=True,
                              timeout=30).stdout
    metadata = dict(line.split(": ", 1) for line in compiler.splitlines()
                    if ": " in line)
    if metadata.get("commit-hash") != selection["build_selection"]["rustc_commit"]:
        raise RuntimeError("Compiler differs from frozen selection")
    if metadata.get("host") not in selection["build_selection"]["targets"]:
        raise RuntimeError("Native host is outside the frozen target selection")
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-line-tail-witness-") as temporary:
        workspace = Path(temporary)
        (workspace / "src").mkdir()
        (workspace / "src/main.rs").write_text(assembled_source(selection))
        for context in selection["build_selection"]["contexts"]:
            (workspace / "Cargo.toml").write_text(manifest(package, context))
            command = [args.cargo, "run", "--offline", "--quiet",
                       "--manifest-path", str(workspace / "Cargo.toml"),
                       "--target", metadata["host"]]
            if args.target_dir:
                command += ["--target-dir", str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=environment, text=True,
                                       capture_output=True, timeout=args.timeout,
                                       cwd=workspace)
            if completed.returncode:
                raise RuntimeError(completed.stderr)
            result = json.loads(completed.stdout)
            if result != {"pointer_width": 64, "valid_queries": 1186569,
                          "invalid_queries": 18468, "lifetime_queries": 4617}:
                raise RuntimeError(f"Unexpected or incomplete oracle counts: {result}")
            records.append({"context": context["name"], **result,
                            "mismatches": 0})
    return {"native_host": metadata["host"], "rustc": metadata,
            "contexts": records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-root", type=Path)
    parser.add_argument("--run-native", action="store_true",
                        help="compile/run assembled witness, only when serialized")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    selection = json.loads(Path(__file__).with_name("selection.json").read_text())
    package = (args.package_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, package)
    result = {"identity": "rstyle.original-line-tail-witness/1",
              "scope": "assembled exact-source private witness; no whole-crate admission claim",
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
