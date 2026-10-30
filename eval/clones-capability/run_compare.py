#!/usr/bin/env python3
"""Reevaluate saved source-rule comparisons; live reference capture is retired.

Pass --legacy, --new, --scope, --expectations and --output as for compare.py.
Recorded inputs keep their original compilation context and metric version.
"""
import json
from pathlib import Path
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from typing import Callable

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent


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
        raise ValueError("recorded reference capture has no facts roster")
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


def main(fixture=None, description=__doc__):
    """Reevaluate retained comparison inputs without executing a retired engine."""
    fixture = fixture or source_fixture()
    return subprocess.call([sys.executable, str(fixture.comparator), *sys.argv[1:]])


if __name__ == "__main__":
    raise SystemExit(main())
