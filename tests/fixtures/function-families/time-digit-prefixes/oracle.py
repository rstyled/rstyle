#!/usr/bin/env python3
"""Verify frozen XXH3/source identities; optionally test unchanged private originals."""
import argparse
import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def checksum(data):
    library = ctypes.util.find_library("xxhash")
    if library is None:
        raise RuntimeError("libxxhash with XXH3_64bits is required for retained identities")
    function = ctypes.CDLL(library).XXH3_64bits
    function.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    function.restype = ctypes.c_uint64
    return f"xxh3-64:{function(data, len(data)):016x}"


def verify_sources(selection, root):
    for relative, identity in selection["package"]["file_identities"].items():
        if checksum((root / relative).read_bytes()) != identity:
            raise ValueError(f"Frozen original file identity changed: {relative}")
    for region in selection["functions"] + selection["declarations"]:
        data = (root / region["file"]).read_bytes()[region["start"]:region["end"]]
        if data != region["source"].encode():
            raise ValueError(f"Frozen original source changed: {region['name']}")
        if "identity" in region and checksum(data) != region["identity"]:
            raise ValueError(f"Frozen original function identity changed: {region['name']}")


def environment(selection):
    result = dict(os.environ, RUSTUP_TOOLCHAIN=selection["build"]["toolchain"])
    for key in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET", "RUSTC_BOOTSTRAP"]:
        result.pop(key, None)
    for key in list(result):
        if key.startswith("CARGO_TARGET_") and key.endswith("_RUSTFLAGS"):
            result.pop(key)
    result.update(CARGO_PROFILE_DEV_OPT_LEVEL="0", CARGO_PROFILE_DEV_DEBUG_ASSERTIONS="true",
                  CARGO_PROFILE_DEV_OVERFLOW_CHECKS="true", CARGO_PROFILE_DEV_PANIC="unwind")
    return result


def compiler_identity(args, selection, env):
    command = [args.rustc, "-vV"]
    completed = subprocess.run(command, env=env, text=True, capture_output=True, check=True)
    fields = dict(line.split(": ", 1) for line in completed.stdout.splitlines() if ": " in line)
    if fields.get("commit-hash") != selection["build"]["rustc_commit"]:
        raise ValueError("Oracle compiler differs from the frozen build selection")
    if fields.get("host") != selection["build"]["target"]:
        raise ValueError("Oracle host differs from this frozen native target; freeze a separate context")
    return fields


def instrumented_files(harness):
    return {
        Path("src/parsing/combinator/mod.rs"): b"\n\n" + harness.read_bytes(),
        Path("src/lib.rs"): b'''\n\n#[doc(hidden)]
#[cfg(feature = "parsing")]
pub fn __rstyle_run_digit_prefix_oracle() {
    parsing::combinator::rstyle_digit_prefix_oracle::run();
}
''',
    }


def verify_copy(root, destination, appended):
    for path in root.rglob("*"):
        relative = path.relative_to(root)
        if path.is_file() and "target" not in relative.parts and ".git" not in relative.parts:
            expected = path.read_bytes() + appended.get(relative, b"")
            if (destination / relative).read_bytes() != expected:
                raise ValueError(f"Temporary copy changed original bytes: {relative}")


def copy_original(root, destination, harness):
    shutil.copytree(root, destination, ignore=shutil.ignore_patterns("target", ".git"))
    for path in destination.rglob("*"):
        if path.is_file():
            path.chmod(path.stat().st_mode | 0o200)
    appended = instrumented_files(harness)
    for relative, extra in appended.items():
        (destination / relative).write_bytes((root / relative).read_bytes() + extra)
    # The package manifest, lockfile and all original files remain unchanged.
    verify_copy(root, destination, appended)


def prepare_runner(directory):
    (directory / "src").mkdir(parents=True)
    (directory / "Cargo.toml").write_text('''[package]
name = "original_time_digit_prefix_oracle"
version = "0.0.0"
edition = "2024"
[workspace]
[features]
default = ["time/default"]
parsing = ["time/parsing"]
[dependencies.time]
path = "../time"
default-features = false
[profile.dev]
opt-level = 0
overflow-checks = true
debug-assertions = true
panic = "unwind"
''')
    (directory / "src/main.rs").write_text(
        "fn main() { time::__rstyle_run_digit_prefix_oracle(); }\n")


def run_context(args, selection, directory, context, env):
    command = [args.cargo, "run", "--offline", "--quiet",
               "--manifest-path", str(directory / "Cargo.toml"),
               "--target-dir", str(args.target_dir.resolve() if args.target_dir else directory / "target")]
    if not context["default_features"]:
        command.append("--no-default-features")
    command += ["--features", "parsing"]
    completed = subprocess.run(command, env=env, text=True, capture_output=True, timeout=args.timeout)
    if completed.returncode:
        raise RuntimeError(f"Oracle failed in {context['name']}:\n{completed.stdout}\n{completed.stderr}")
    marker = "RSTYLE_DIGIT_PREFIX_ORACLE "
    records = [json.loads(line.split(marker, 1)[1]) for line in completed.stdout.splitlines() if marker in line]
    if len(records) != 1 or records[0].get("mismatches") != 0:
        raise ValueError(f"Expected one successful original oracle record: {completed.stdout}")
    expected = {"one_byte_inputs": 256, "two_byte_inputs": 65536, "valid_three_digit_prefixes": 1000,
                "one_byte_checks": 768, "two_byte_checks": 196608, "valid_and_suffix_checks": 21768,
                "invalid_position_checks": 8856, "short_checks": 1536, "pointer_checks": 18,
                "total_checks": 229554}
    if any(records[0].get(key) != value for key, value in expected.items()):
        raise ValueError(f"Oracle case inventory changed: {records[0]}")
    return dict(records[0], context=context["name"], features=context["feature_closure"], command=command)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="coordinator schedules the two original-crate builds")
    parser.add_argument("--context", choices=["both", "default-parsing", "no-default-parsing"], default="both")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--timeout", type=int, default=900)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    selection = json.loads((here / "selection.json").read_text())
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, root)
    result = {"identity": "rstyle.original-time-digit-prefix-oracle/1", "source_identities_checked": True,
              "expected_source_classification": "in-scope", "admission_evaluated": False, "contexts": []}
    if args.run_native:
        env = environment(selection)
        env["RUSTC"] = args.rustc
        result["compiler"] = compiler_identity(args, selection, env)
        with tempfile.TemporaryDirectory(prefix="rstyle-time-digit-prefix-oracle-") as temporary:
            directory = Path(temporary) / "time"
            copy_original(root, directory, here / "oracle.rs")
            runner = Path(temporary) / "runner"
            prepare_runner(runner)
            for context in selection["build"]["contexts"]:
                if args.context == "both" or args.context == context["name"]:
                    result["contexts"].append(run_context(args, selection, runner, context, env))
            verify_copy(root, directory, instrumented_files(here / "oracle.rs"))
            verify_sources(selection, root)
    result["native_execution"] = bool(result["contexts"])
    result["limitation"] = "Native unchanged-method behavior only; capture, strict admission and matching are separate."
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
