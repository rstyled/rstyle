#!/usr/bin/env python3
"""Run serial same-scope legacy/new acceptance captures using prebuilt binaries.

Run only after checkpoint builds/gates have stopped. Output must not exist.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass
from typing import Callable

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def controlled_environment(output, toolchain, driver):
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("RSTYLE_", "CARGO_PROFILE_"))}
    for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER",
                "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTFLAGS"):
        env.pop(key, None)
    env.update(RUSTUP_TOOLCHAIN=toolchain, CARGO_NET_OFFLINE="true",
               CARGO_TARGET_DIR=str(output), RSTYLE_DRIVER=str(driver))
    return env


def capture(output, name, command, env, timeout, accepted=(0,)):
    """Retain command/status/logs even on timeout, then refuse failed captures."""
    record = {"command": [str(arg) for arg in command], "cwd": str(ROOT),
              "environment": {key: env[key] for key in ("RUSTUP_TOOLCHAIN", "CARGO_NET_OFFLINE", "CARGO_TARGET_DIR", "RSTYLE_DRIVER")},
              "timeout_seconds": timeout}
    started = time.monotonic()
    with (output / (name + ".stdout")).open("wb") as stdout, (output / (name + ".stderr")).open("wb") as stderr:
        process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr,
                                   start_new_session=True)
        try:
            record["exit_code"] = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            record["execution"] = "timed_out"
            record["exit_code"] = process.returncode
    record["elapsed_seconds"] = time.monotonic() - started
    write_json(output / (name + ".command.json"), record)
    if record.get("execution") == "timed_out" or record["exit_code"] not in accepted:
        raise RuntimeError(name + " failed; retained logs at " + str(output))
    return (output / (name + ".stdout")).read_text()


FIXTURE_EXCLUDES = ["target/**", "src/excluded.rs"]


def fixture_policy(policy):
    configured = tomllib.loads(policy.read_text())
    expected = {"exclude": FIXTURE_EXCLUDES,
                "fn_length": {"max_lines": 5},
                "fn_params": {"max_params": 4, "count_self": False},
                "fn_spacing": {"min_statements": 5}}
    if configured != expected:
        raise ValueError("runner expects fixed acceptance source-rule policy, including exclusions")
    return {"max_lines": configured["fn_length"]["max_lines"],
            "max_params": configured["fn_params"]["max_params"],
            "count_self": configured["fn_params"]["count_self"],
            "min_statements": configured["fn_spacing"]["min_statements"],
            "exclude": configured["exclude"]}


def roster(facts_file, legacy, workspace_root, excluded_owner="excluded::over_both_limits"):
    facts = json.loads(facts_file.read_text())
    if not isinstance(facts, list) or not facts:
        raise ValueError("fresh legacy capture has no facts roster")
    result = []
    for function in facts:
        if not all(key in function for key in ("name", "path", "line")):
            raise ValueError("malformed captured legacy function")
        source_path = Path(function["path"])
        original_path = source_path if source_path.is_absolute() else workspace_root / source_path
        excluded = original_path == workspace_root / "src/excluded.rs"
        result.append({"name": function["name"], "source_file": function["path"],
                       "line": function["line"], "excluded": excluded})
    excluded = [function for function in result if function["excluded"]]
    expected_name = excluded_owner
    if len(excluded) != 1 or not (excluded[0]["name"] == expected_name
                                  or excluded[0]["name"].endswith("::" + expected_name)):
        raise ValueError("captured facts must retain the fixed excluded acceptance function")
    if legacy.get("summary", {}).get("functions") != sum(not function["excluded"] for function in result):
        raise ValueError("legacy summary count differs from captured facts; completeness unproven")
    return result


def commands(binary, manifest, policy, package="clones-capability-acceptance", analysis="source", new_policy=None):
    shared = [str(binary), "rstyle", "--manifest-path", str(manifest), "--package",
              package, "--format", "json"]
    old = shared + ["--style-engine", "legacy", "--config", str(policy)]
    flags = (["--effect-density", "--effect-config"] if analysis == "density"
             else ["--source-rules", "--analysis-config"])
    new = shared + ["clones", "--extraction-profile", "hir-data-v3", "--locked",
                    "--report-detail", "compact"] + flags + [str(new_policy or policy)]
    return old, new


def fixture_inputs(manifest, policy, lockfile):
    return (manifest, policy, lockfile, HERE / "expectations.json",
            manifest.parent / "src/lib.rs", manifest.parent / "src/excluded.rs",
            manifest.parent / "src/spacing.rs")


@dataclass(frozen=True)
class Fixture:
    manifest: Path
    policy: Path
    expectations: Path
    package: str
    analysis: str
    excluded_owner: str
    policy_reader: Callable[[Path], dict]
    inputs: tuple
    comparator: Path
    new_policy: Path | None = None
    new_policy_reader: Callable[[Path], dict] | None = None


def source_fixture():
    manifest = HERE / "corpus/Cargo.toml"
    policy = HERE / "analysis.toml"
    return Fixture(manifest, policy, HERE / "expectations.json", "clones-capability-acceptance",
                   "source", "excluded::over_both_limits", fixture_policy,
                   fixture_inputs(manifest, policy, manifest.with_name("Cargo.lock")) +
                   (HERE / "run_compare.py", HERE / "compare.py"), HERE / "compare.py")


def run(args, fixture=None):
    fixture = fixture or source_fixture()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    driver = args.driver.resolve()
    for executable in (binary, driver):
        if not executable.is_file():
            raise ValueError("missing prebuilt executable: " + str(executable))
    manifest = fixture.manifest
    policy = fixture.policy
    effective_policy = fixture.policy_reader(policy)
    if fixture.new_policy is not None:
        if fixture.new_policy_reader is None or fixture.new_policy_reader(fixture.new_policy) != effective_policy:
            raise ValueError("legacy/new fixture effective policies differ")
    lockfile = manifest.parent / "Cargo.lock"
    lock_before = lockfile.read_bytes()
    package = tomllib.loads(manifest.read_text())
    if package["package"]["name"] != fixture.package or package.get("features"):
        raise ValueError("runner expects fixed feature-free acceptance package")
    if list((manifest.parent / "src").glob("bin/*")) or (manifest.parent / "src/main.rs").exists():
        raise ValueError("legacy/new scope requires library-only fixture")
    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    env = controlled_environment(output / "legacy-target", toolchain, driver)
    revision = capture(output, "revision", ["git", "rev-parse", "HEAD"], env, 10).strip()
    capture(output, "worktree-status", ["git", "status", "--short"], env, 10)
    capture(output, "worktree-diff", ["git", "diff", "--binary", "HEAD"], env, 10)
    capture(output, "compiler", ["rustc", "--version", "--verbose"], env, 10)
    capture(output, "cargo", ["cargo", "--version"], env, 10)
    snapshots = output / "inputs"
    snapshots.mkdir()
    for source in fixture.inputs:
        target = snapshots / source.relative_to(HERE)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
    write_json(output / "executables.json", [{"path": str(path), "size": path.stat().st_size,
                                              "mtime_ns": path.stat().st_mtime_ns} for path in (binary, driver)])
    old_command, new_command = commands(binary, manifest, policy, fixture.package, fixture.analysis, fixture.new_policy)
    legacy = json.loads(capture(output, "legacy", old_command, env, args.timeout, (0, 1)))
    facts_file = output / ("legacy-target/rstyle/facts/" + fixture.package.replace("-", "_") + "-lib.json")
    functions = roster(facts_file, legacy, manifest.parent, fixture.excluded_owner)
    new_env = controlled_environment(output / "new-target", toolchain, driver)
    new = json.loads(capture(output, "new", new_command, new_env, args.timeout, (0, 2) if fixture.analysis == "density" else (0,)))
    if lockfile.read_bytes() != lock_before:
        raise ValueError("fixture lockfile changed during captures")
    context = {"manifest": str(manifest), "package": fixture.package, "target": "lib",
               "features": [], "profile": "dev", "toolchain": toolchain, "revision": revision,
               "policy": effective_policy}
    scope = {"legacy": {"context": context, "complete": True, "functions": functions},
             "new": {"context": context}, "limitations": [
                 "Legacy CLI lacks --locked/--lib/features/profile; fixed library-only fixture, offline lockfile unchanged, empty features and dev host defaults used.",
                 "Prebuilt binary provenance is recorded by path/stat, not proven by Git revision; build checkpoint before this run."]}
    write_json(output / "scope.json", scope)
    write_json(output / "legacy.json", legacy)
    write_json(output / "new.json", new)
    comparison = [sys.executable, str(fixture.comparator), "--legacy", str(output / "legacy.json"),
                  "--new", str(output / "new.json"), "--scope", str(output / "scope.json"),
                  "--expectations", str(fixture.expectations), "--output", str(output / "comparison.json")]
    capture(output, "compare", comparison, env, 10)
    return 0


def main(fixture=None, description=__doc__):
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument("--output", required=True, type=Path, help="new retained artifact directory")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/cargo-rstyle")
    parser.add_argument("--driver", type=Path, default=ROOT / "target/debug/rstyle-driver")
    parser.add_argument("--timeout", type=int, default=180, help="seconds per capture; no automatic retries")
    args = parser.parse_args()
    if args.timeout < 1:
        parser.error("timeout must be positive")
    try:
        return run(args, fixture)
    except (OSError, ValueError, RuntimeError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
