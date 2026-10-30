#!/usr/bin/env python3
"""Execute the unchanged hour/min wrappers against independent decimal/suffix expectations."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


def common_module():
    path = Path(__file__).resolve().parent.parent / "time-digit-prefixes/oracle.py"
    spec = importlib.util.spec_from_file_location("original_prefix_oracle", path)
    common = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(common)
    return common


def run_context(args, selection, runner, context, env):
    command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
               str(runner / "Cargo.toml"), "--target-dir", str(args.target_dir.resolve()),
               "--features", "parsing"]
    if not context["default_features"]:
        command.append("--no-default-features")
    completed = subprocess.run(command, env=env, capture_output=True, text=True, timeout=args.timeout)
    if completed.returncode:
        raise RuntimeError(f"{context['name']}: {completed.stdout}\n{completed.stderr}")
    marker = "RSTYLE_DIGIT_FORWARDER_ORACLE "
    records = [json.loads(line.split(marker, 1)[1]) for line in completed.stdout.splitlines() if marker in line]
    if records != [{"total_checks": selection["oracle_cases"]["expected_checks_per_context"], "mismatches": 0}]:
        raise ValueError(f"Unexpected oracle inventory: {records}")
    return dict(records[0], context=context["name"], features=context["feature_closure"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-native", action="store_true")
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--target-dir", type=Path, default=Path("target/time-digit-forwarder-oracle"))
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--timeout", type=int, default=900)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    selection = json.loads((here / "selection.json").read_text())
    common = common_module()
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    common.verify_sources(selection, root)
    result = {"identity": "rstyle.original-time-digit-forwarder-oracle/1",
              "source_identities_checked": True, "admission_evaluated": False, "contexts": []}
    if args.run_native:
        env = common.environment(selection)
        env["RUSTC"] = args.rustc
        result["compiler"] = common.compiler_identity(args, selection, env)
        with tempfile.TemporaryDirectory(prefix="rstyle-time-digit-forwarders-") as temporary:
            copied = Path(temporary) / "time"
            common.copy_original(root, copied, here / "oracle.rs")
            runner = Path(temporary) / "runner"
            common.prepare_runner(runner)
            for context in selection["build"]["contexts"]:
                result["contexts"].append(run_context(args, selection, runner, context, env))
            common.verify_copy(root, copied, common.instrumented_files(here / "oracle.rs"))
            common.verify_sources(selection, root)
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
