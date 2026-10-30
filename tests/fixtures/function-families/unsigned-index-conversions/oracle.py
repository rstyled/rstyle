#!/usr/bin/env python3
"""Source-first unsigned TryFrom family; original public runtime oracle, no admission."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile


def common_module(here):
    spec = importlib.util.spec_from_file_location("index_source_checks", here.parent / "checked-index-decode/oracle.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_native(selection, root, source, context, debug):
    compiler = Path.home() / ".cargo/bin/rustc"
    toolchain = selection["build_selection"]["toolchain"]
    version = subprocess.check_output([str(compiler), "+" + toolchain, "-vV"], text=True)
    fields = dict(line.split(": ", 1) for line in version.splitlines() if ": " in line)
    assert fields["commit-hash"] == selection["build_selection"]["rustc_commit"]
    assert fields["host"] in selection["build_selection"]["targets"]
    environment = {k: v for k, v in os.environ.items()
                   if not k.startswith(("RSTYLE_", "CARGO_PROFILE_", "CARGO_TARGET_"))
                   and k not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET"}}
    environment["RUSTUP_TOOLCHAIN"] = toolchain
    with tempfile.TemporaryDirectory(prefix="rstyle-unsigned-index-oracle-") as temporary:
        directory = Path(temporary)
        (directory / "src").mkdir()
        (directory / "Cargo.toml").write_text(
            '[package]\nname="unsigned_index_oracle"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
            '[dependencies]\n' + f'regex-automata={{path={json.dumps(str(root))},default-features={str(context["default_features"]).lower()}}}\n'
            '[profile.dev]\nopt-level=0\noverflow-checks=true\npanic="unwind"\n'
            + f'debug-assertions={str(debug).lower()}\n')
        (directory / "src/main.rs").write_text(source)
        output = subprocess.check_output([str(Path.home() / ".cargo/bin/cargo"), "+" + toolchain,
            "run", "--offline", "--quiet", "--manifest-path", str(directory / "Cargo.toml"),
            "--target-dir", str(Path.cwd() / "target/unsigned-index-oracle")],
            env=environment, text=True, timeout=180)
    result = json.loads(output)
    result.update(context=context["name"], host=fields["host"], admission_evaluated=False)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-native", action="store_true")
    parser.add_argument("--context", choices=["default", "no-default", "both"], default="both")
    parser.add_argument("--debug-mode", choices=["debug", "release-branch", "both"], default="both")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    selection = json.loads((here / "selection.json").read_text())
    root = Path(selection["package"]["source_root"])
    common = common_module(here)
    common.verify_sources(selection, root)
    result = {"identity": "rstyle.original-unsigned-index-oracle/1", "source_identities_checked": True,
              "admission_evaluated": False, "contexts": []}
    if args.run_native:
        modes = [True, False] if args.debug_mode == "both" else [args.debug_mode == "debug"]
        for context in selection["build_selection"]["contexts"]:
            if args.context in ("both", context["name"]):
                for debug in modes:
                    result["contexts"].append(run_native(selection, root, (here / "oracle.rs").read_text(), context, debug))
    common.verify_sources(selection, root)
    text = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(text)
    print(text, end="")


if __name__ == "__main__":
    main()
