"""Run source/HIR/paired-document capture, with no pair prediction evaluation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("stage")
    parser.add_argument("profile")
    args = parser.parse_args()
    workspace = Path(__file__).resolve().parents[2]
    binary = workspace / "target/debug/cargo-rstyle"
    driver = workspace / "target/debug/rstyle-driver"
    selection = json.loads(Path(__file__).with_name("selection.json").read_text())
    evidence = []
    for name, version in sorted(set((f["package"], f["version"]) for f in selection)):
        package = args.root / f"{name}-{version}"
        command = [str(binary), "rstyle", "--manifest-path", str(package / "Cargo.toml"),
                   "clones", "--format", "json", "--max-pairs", "0", "--extraction-profile", args.profile]
        output = subprocess.run(command, env=dict(os.environ, RSTYLE_DRIVER=str(driver)), capture_output=True)
        (package / f"{args.stage}.stdout.json").write_bytes(output.stdout)
        (package / f"{args.stage}.stderr.txt").write_bytes(output.stderr)
        report = json.loads(output.stdout) if output.stdout else {}
        evidence.append(dict(package=name, version=version, command=command, exit_code=output.returncode,
                             report_sha256=hashlib.sha256(output.stdout).hexdigest(),
                             stderr_sha256=hashlib.sha256(output.stderr).hexdigest(),
                             driver_sha256=hashlib.sha256(driver.read_bytes()).hexdigest(),
                             cargo_rstyle_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                             capture_complete=report.get("capture_complete"),
                             assessment_complete=report.get("assessment_complete"),
                             capture_failures=report.get("capture_failures"),
                             handoff_failures=report.get("handoff_failures"),
                             run_directory=report.get("run_directory")))
        print(name, output.returncode, len(output.stdout), flush=True)
    Path(__file__).with_name(f"{args.stage}-commands.json").write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
