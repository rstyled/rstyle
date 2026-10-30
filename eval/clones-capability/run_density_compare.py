#!/usr/bin/env python3
"""Reevaluate saved density comparisons without a live reference engine."""
import json
import tomllib

from run_compare import Fixture, HERE, main
from density_compare import policy_error


def fixture_policy(path, legacy=False):
    configured = tomllib.loads(path.read_text())
    if not legacy and "transitive_depth" in configured["side_effects"]:
        raise ValueError("density frontend fixes call depth internally; omit transitive_depth")
    policy = dict(configured["side_effects"], exclude=configured["exclude"])
    if not legacy:
        policy["transitive_depth"] = 2
    expected = json.loads((HERE / "density-comparison-cases.json").read_text())["scope"]["policy"]
    if policy_error(policy) or policy != expected or set(configured) != {"exclude", "side_effects"}:
        raise ValueError("density runner requires the independently labeled full policy")
    return policy


def density_fixture():
    manifest = HERE / "density-corpus/Cargo.toml"
    policy = HERE / "density-legacy-policy.toml"
    new_policy = HERE / "density-policy.toml"
    expectations = HERE / "density-comparison-cases.json"
    inputs = (manifest, manifest.with_name("Cargo.lock"), policy, new_policy, expectations,
              manifest.parent / "src/lib.rs", manifest.parent / "src/excluded.rs",
              HERE / "run_density_compare.py", HERE / "run_compare.py",
              HERE / "density_compare.py", HERE / "compare.py")
    return Fixture(manifest, policy, expectations, "density-capability-acceptance",
                   "density", "excluded::write", lambda path: fixture_policy(path, legacy=True), inputs,
                   HERE / "density_compare.py", new_policy, fixture_policy)


if __name__ == "__main__":
    raise SystemExit(main(density_fixture(), __doc__))
