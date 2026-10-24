# /// script
# requires-python = ">=3.12"
# dependencies = ["tree-sitter==0.25.2", "tree-sitter-rust==0.24.0", "jsonschema==4.25.1", "xxhash==3.6.0"]
# ///
"""Recreate the frozen eligibility inventory; generated evidence stays in target/."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib
import urllib.request

import jsonschema
from tree_sitter import Language, Parser
import tree_sitter_rust

import assess
import summarize
from bundle_records import verify_documents

ROOT = Path(__file__).resolve().parent
WORKSPACE = ROOT.parents[1]
HOST = "aarch64-unknown-linux-gnu"
COMPILER = "5c543b0b8c73c7b72bc8284ced4fb22ead15734d"
PROFILES = ("r0", "hir-expanded-v1", "hir-data-v3")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def clean_environment():
    # Keep PATH, registry credentials and proxies, but isolate build selection.
    blocked = {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER",
               "RUSTC_WORKSPACE_WRAPPER", "RUSTUP_TOOLCHAIN", "CARGO_TARGET_DIR"}
    return {key: value for key, value in os.environ.items()
            if key not in blocked and not key.startswith((
                "RSTYLE_", "CARGO_BUILD_", "CARGO_PROFILE_", "CARGO_TARGET_"))}


def inventory(package, name, version):
    parser = Parser(Language(tree_sitter_rust.language()))
    functions = []
    for path in sorted((package / "src").rglob("*.rs")):
        source = path.read_bytes()

        def visit(node):
            if node.type == "function_item":
                exact = source[node.start_byte:node.end_byte]
                label = node.child_by_field_name("name")
                functions.append(dict(
                    package=name, version=version, path=path.relative_to(package).as_posix(),
                    start=node.start_byte, end=node.end_byte,
                    name=source[label.start_byte:label.end_byte].decode(),
                    source=exact.decode(), sha256=digest(exact)))
            for child in node.children:
                visit(child)

        visit(parser.parse(source).root_node)
    return sorted(functions, key=lambda item: (item["path"], item["start"]))[:24]


def prepare_packages(destination, provenance, selection):
    if digest((ROOT / "selection.json").read_bytes()) != provenance["selection_sha256"]:
        raise ValueError("Frozen selection digest changed")
    recreated = []
    for item in provenance["packages"]:
        with urllib.request.urlopen(item["archive"], timeout=60) as response:
            archive = response.read()
        if digest(archive) != item["archive_sha256"]:
            raise ValueError(f"Archive digest mismatch: {item['package']}")
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as archive_file:
            archive_file.extractall(destination, filter="data")
        package = destination / f"{item['package']}-{item['version']}"
        recreated.extend(inventory(package, item["package"], item["version"]))
        lock = (ROOT / item["lockfile"]).read_bytes()
        if digest(lock) != item["lockfile_sha256"]:
            raise ValueError(f"Lockfile digest mismatch: {item['package']}")
        (package / "Cargo.lock").write_bytes(lock)
    if recreated != selection:
        raise ValueError("Downloaded source does not reproduce the frozen selection")


def check_capture(report, exit_code):
    if exit_code not in (0, 2):
        raise ValueError(f"Unexpected inventory command exit: {exit_code}")
    if not all(report.get(key) is True for key in ("capture_complete", "assessment_complete")):
        raise ValueError("Incomplete source/HIR capture or assessment")
    if report.get("capture_failures") or report.get("handoff_failures"):
        raise ValueError("Compiler capture or serialized handoff failed")
    matching = report.get("matching", {})
    if matching.get("pairs"):
        raise ValueError("Eligibility inventory must not evaluate detector predictions")
    diagnostics = matching.get("diagnostics", [])
    if exit_code == 0 and (diagnostics or matching.get("incomplete") is not False):
        raise ValueError("Successful inventory command reported incomplete matching or errors")
    if exit_code == 2 and (len(diagnostics) != 1 or
                          not diagnostics[0].startswith("pair enumeration requires ") or
                          not diagnostics[0].endswith("exceeds explicit resource limit 0")):
        raise ValueError("Exit 2 was not the explicit zero-pair inventory budget")


def capture(package, profile, output, env, validators):
    directory = output / profile / package.name
    directory.mkdir(parents=True)
    env = dict(env, CARGO_TARGET_DIR=str(directory / "target"),
               RSTYLE_DRIVER=str(WORKSPACE / "target/debug/rstyle-driver"))
    command = [str(WORKSPACE / "target/debug/cargo-rstyle"), "rstyle",
               "--manifest-path", str(package / "Cargo.toml"), "clones", "--locked",
               "--format", "json", "--report-detail", "compact", "--max-pairs", "0",
               "--extraction-profile", profile]
    with (directory / "stdout.json").open('wb') as stdout, (directory / "stderr.txt").open('wb') as stderr:
        result = subprocess.run(command, cwd=package, env=env, stdout=stdout, stderr=stderr)
    write_json(directory / "command.json", dict(command=command, exit_code=result.returncode))
    with (directory / "stdout.json").open() as stdout:
        report = json.load(stdout)
    if report.get('report_detail') != 'compact':
        raise ValueError('Inventory command did not produce requested compact report')
    check_capture(report, result.returncode)
    checked = verify_documents(report, validators)
    print(f"{profile}: {package.name}: {checked} verified documents", flush=True)
    return report


def reproduce(output, profiles, env):
    provenance = json.loads((ROOT / "provenance.json").read_text())
    selection = json.loads((ROOT / "selection.json").read_text())
    toolchain = tomllib.loads((WORKSPACE / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    compiler = subprocess.check_output(
        ["rustup", "run", toolchain, "rustc", "-vV"], env=env, text=True)
    if f"host: {HOST}\n" not in compiler or f"commit-hash: {COMPILER}\n" not in compiler:
        raise ValueError("Inventory requires the frozen aarch64 compiler; cross-target evidence is deferred")
    write_json(output / "run.json", dict(
        revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=WORKSPACE, text=True).strip(),
        working_tree=subprocess.check_output(["git", "status", "--porcelain"], cwd=WORKSPACE, text=True),
        compiler=compiler, profiles=profiles))
    subprocess.run(["cargo", f"+{toolchain}", "build", "--locked", "-p", "cargo-rstyle",
                    "-p", "rstyle-driver"], cwd=WORKSPACE,
                   env=dict(env, CARGO_TARGET_DIR=str(WORKSPACE / "target")), check=True)
    vendor = WORKSPACE / "vendor-compat/rust-r0"
    validators = {kind: jsonschema.Draft202012Validator(
        json.loads((vendor / f"{kind}.schema.json").read_text())) for kind in ("pdg", "cfg")}
    stages = []
    # Outside the repository so Cargo does not adopt its enclosing workspace.
    with tempfile.TemporaryDirectory(prefix="rstyle-coverage-sources-") as temporary:
        sources = Path(temporary)
        prepare_packages(sources, provenance, selection)
        for profile in profiles:
            reports = {}
            for item in provenance["packages"]:
                package = sources / f"{item['package']}-{item['version']}"
                reports[item["package"]] = capture(package, profile, output, env, validators)
                if digest((package / "Cargo.lock").read_bytes()) != item["lockfile_sha256"]:
                    raise ValueError("Capture changed a pinned dependency lockfile")
            stage = assess.assess(selection, reports)
            stage["stage"] = profile
            stages.append(stage)
            write_json(output / f"{profile}.json", stage)
            print(f"{profile}: {stage['counts']}", flush=True)
    write_json(output / "summary.json", summarize.summarize(stages))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles", nargs="+", choices=PROFILES, default=list(PROFILES))
    args = parser.parse_args()
    parent = WORKSPACE / "target/hir-coverage"
    parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    print(f"Generated results: {output}", flush=True)
    try:
        reproduce(output, args.profiles, clean_environment())
    except Exception as error:
        (output / "failure.txt").write_text(str(error) + "\n")
        raise


if __name__ == "__main__":
    main()
