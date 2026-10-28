#!/usr/bin/env python3
"""Compare static density reports with independent numeric and coverage labels."""
import argparse
from collections import Counter
import json
from pathlib import Path

from compare import context_errors, legacy_assessment, named, nonnegative_integer

LABEL_SCHEMA = "rstyle.clones-capability.density-comparison-cases/1"
RULE = "side_effect_density"
KINDS = frozenset(("mutation", "io", "network", "global", "unsafe", "time", "random", "panic", "unknown"))
PATH_KINDS = frozenset(("io", "network", "time", "random", "panic"))
SUCCESSES = frozenset(("evaluated-agreement", "intentional-difference", "expected-uncovered", "policy-excluded-agreement"))


def policy_error(policy):
    if not isinstance(policy, dict) or set(policy) != {"max_density_milli", "min_statements", "min_lines", "transitive_depth", "weights", "paths", "exclude"}:
        return "effective density policy missing or malformed"
    if any(not nonnegative_integer(policy[key]) for key in ("max_density_milli", "min_statements", "min_lines", "transitive_depth")) or policy["transitive_depth"] != 2:
        return "density numeric policy malformed or unsupported"
    weights, paths = policy["weights"], policy["paths"]
    if not isinstance(weights, dict) or set(weights) != KINDS or any(not nonnegative_integer(value) for value in weights.values()):
        return "complete density weights missing or malformed"
    if not isinstance(paths, dict) or set(paths) != PATH_KINDS or any(not string_list(value) for value in paths.values()) or not string_list(policy["exclude"]):
        return "complete density paths or exclusions missing or malformed"
    return None


def string_list(value):
    return isinstance(value, list) and all(isinstance(item, str) and item for item in value)


def counts_valid(counts):
    return isinstance(counts, dict) and set(counts) <= KINDS and all(nonnegative_integer(count) and count > 0 for count in counts.values())


def label_error(case, policy):
    expected = case.get("expected", {})
    if not isinstance(expected, dict) or not counts_valid(expected.get("events")):
        return "independent event counts missing or malformed"
    if any(type(expected.get(key, False)) is not bool for key in ("complete", "exempt")) or "complete" not in expected:
        return "independent completeness or exemption malformed"
    if any(type(case.get(key, False)) is not bool for key in ("suppressed", "excluded")):
        return "independent source policy label malformed"
    if any(not nonnegative_integer(expected.get(key)) for key in ("statements", "code_lines")):
        return "independent source denominator malformed"
    if not expected["complete"]:
        if expected.get("actual") is not None or expected.get("outcome") != "not_evaluated" or expected.get("exempt", False):
            return "incomplete label cannot authorize numeric or clean results"
        return None
    if expected["events"].get("unknown", 0):
        return "unknown events cannot authorize complete labels"
    exempt = expected["statements"] < policy["min_statements"] or expected["code_lines"] < policy["min_lines"]
    actual = None if exempt else sum(count * policy["weights"][kind] for kind, count in expected["events"].items()) * 1000 // max(1, expected["statements"])
    outcome = "finding" if actual is not None and actual > policy["max_density_milli"] else "evaluated_no_finding"
    if (expected.get("exempt", False) != exempt or expected.get("actual") != actual
            or (actual is not None and not nonnegative_integer(expected.get("actual"))) or expected.get("outcome") != outcome):
        return "independent label contradicts density arithmetic or policy"
    return None


def new_assessment(report, name, policy):
    if report.get("capture_complete") is not True or report.get("assessment_complete") is not True:
        return None, "top-level capture or handoff assessment incomplete"
    analysis = report.get("mutation_effects", {})
    if not isinstance(analysis, dict):
        return None, "density report malformed"
    if analysis.get("schema") != "rstyle.mutation-effects/4" or analysis.get("density_requested") is not True:
        return None, "requested density report missing or unsupported"
    if analysis.get("execution") not in ("completed", "budget_exhausted"):
        return None, "density capture or execution unavailable"
    if analysis.get("statement_metric") != "rstyle.original-source-statements/1" or analysis.get("density_metric") != "rstyle.static-effect-events/1":
        return None, "density metric unsupported or missing"
    config = analysis.get("density_config", {})
    if not isinstance(config, dict) or not isinstance(config.get("side_effects"), dict):
        return None, "density configuration malformed"
    effective = dict(config["side_effects"], exclude=config.get("exclude"))
    # Python bool compares equal to integers; validate before structural equality.
    error = policy_error(effective)
    if error or effective != policy:
        return None, error or "reported density policy contradicts independent policy"
    functions = analysis.get("functions", [])
    if not isinstance(functions, list):
        return None, "density function roster unavailable"
    matches = [function for function in functions if isinstance(function, dict) and isinstance(function.get("name"), str) and named(function["name"], name)]
    if len(matches) != 1 or not isinstance(matches[0].get("density"), dict):
        return None, "density function missing or ambiguous across owners"
    evidence = matches[0].get("evidence")
    if not isinstance(evidence, dict) or evidence.get("execution") not in ("completed", "budget_exhausted"):
        return None, "original owner assessment unavailable"
    return matches[0], None


def source_policy(density):
    suppression, exclusion = density.get("source_suppression", {}), density.get("source_exclusion", {})
    if not isinstance(suppression, dict) or not isinstance(exclusion, dict):
        return None, "source policy observation malformed"
    if suppression.get("status") not in ("none", "allowed") or exclusion.get("status") not in ("included", "excluded", "not_applicable"):
        return None, "source suppression or exclusion unavailable or invalid"
    if not isinstance(exclusion.get("path"), str) or not exclusion["path"]:
        return None, "source exclusion path missing"
    rules = suppression.get("rules", [])
    if not isinstance(rules, list) or any(not isinstance(rule, str) for rule in rules):
        return None, "source suppression rule roster malformed"
    suppressed = suppression["status"] == "allowed" and RULE in rules
    if type(density.get("suppressed")) is not bool or density["suppressed"] != suppressed:
        return None, "density suppression contradicts source permission"
    return (suppressed, exclusion["status"] == "excluded"), None


def assess_case(legacy, new, case, scope, policy):
    try:
        old, old_error = legacy_assessment(legacy, scope.get("legacy", {}), case["function"], RULE)
    except (KeyError, TypeError, AttributeError):
        old, old_error = None, "legacy function or diagnostic roster malformed"
    function, error = new_assessment(new, case["function"], policy)
    row = {"function": case["function"], "rule_id": RULE, "expected": case.get("expected"), "legacy": old}
    if old_error or error or label_error(case, policy):
        return finish(row, "uncovered", old_error or error or label_error(case, policy))
    density = function["density"]
    row["new"] = density
    expected = case["expected"]
    if type(density.get("complete")) is not bool:
        return finish(row, "uncovered", "density completeness missing or malformed")
    inventory, measurement = density.get("event_inventory", {}), density.get("measurement", {})
    if not isinstance(inventory, dict) or not isinstance(measurement, dict):
        return finish(row, "uncovered", "density inventory or measurement missing")
    if not expected["complete"]:
        if (density["complete"] is False and inventory.get("coverage") == "incomplete"
                and measurement.get("outcome") == "not_evaluated" and "actual" in measurement
                and measurement["actual"] is None and measurement.get("exempt") is False):
            return finish(row, "expected-uncovered", "Declared incomplete evidence; no numeric or clean agreement claimed")
        return finish(row, "actual-regression", "incomplete independent label contradicts reported completeness")
    if (density["complete"] is False or inventory.get("coverage") != "complete"
            or function["evidence"]["execution"] != "completed"):
        return finish(row, "uncovered", "complete density evidence unavailable")
    counts, events = inventory.get("counts"), inventory.get("events")
    if not counts_valid(counts) or not isinstance(events, list) or any(not isinstance(event, dict) or not isinstance(event.get("kind"), str) or event["kind"] not in KINDS for event in events):
        return finish(row, "uncovered", "event counts or original event roster malformed")
    if counts.get("unknown", 0) or inventory.get("unresolved_boundaries") != []:
        return finish(row, "uncovered", "unknown effect boundaries prevent numeric agreement")
    if dict(Counter(event["kind"] for event in events)) != counts:
        return finish(row, "actual-regression", "event roster contradicts event counts")
    observed_policy, error = source_policy(density)
    if error:
        return finish(row, "uncovered", error)
    suppressed, excluded = observed_policy
    if suppressed != case.get("suppressed", False) or excluded != case.get("excluded", False) or old.get("excluded", False) != excluded:
        return finish(row, "actual-regression", "source policy contradicts independent labels")
    source = measurement.get("source", {})
    if not isinstance(source, dict) or any(not nonnegative_integer(source.get(key)) for key in ("statements", "code_lines")):
        return finish(row, "uncovered", "source denominator unavailable or malformed")
    actual = measurement.get("actual")
    if (type(measurement.get("exempt")) is not bool or not nonnegative_integer(measurement.get("limit"))
            or "actual" not in measurement or (actual is not None and not nonnegative_integer(actual))):
        return finish(row, "uncovered", "numeric density measurement malformed")
    if (counts != expected["events"] or any(source[key] != expected[key] for key in ("statements", "code_lines"))
            or actual != expected["actual"] or measurement.get("outcome") != expected["outcome"]
            or measurement["exempt"] != expected.get("exempt", False) or measurement["limit"] != policy["max_density_milli"]):
        return finish(row, "actual-regression", "new density contradicts independent events, denominator, arithmetic or policy")
    visible = expected["outcome"] == "finding" and not suppressed and not excluded
    row["new_visible_outcome"] = "finding" if visible else "evaluated_no_finding"
    if density.get("invalid_suppression") is not None:
        return finish(row, "uncovered", "invalid suppression diagnostic outside numeric comparison")
    if not nonnegative_integer(density.get("visible_findings")) or density["visible_findings"] != int(visible):
        return finish(row, "actual-regression", "density visibility contradicts raw numeric result or source policy")
    if excluded:
        return finish(row, "policy-excluded-agreement")
    if old["outcome"] == row["new_visible_outcome"]:
        if not visible or (nonnegative_integer(old.get("actual")) and nonnegative_integer(old.get("limit")) and old["actual"] == actual and old["limit"] == policy["max_density_milli"]):
            return finish(row, "evaluated-agreement")
    reason = case.get("reviewed_difference")
    if isinstance(reason, str) and reason.strip():
        return finish(row, "intentional-difference", reason)
    return finish(row, "unreviewed-disagreement", "new matches labels; legacy difference requires explicit review")


def finish(row, classification, reason=None):
    row.update(classification=classification, reasons=[reason] if reason else [])
    return row


def compare(legacy, new, expectations, scope):
    malformed_inputs = any(not isinstance(value, dict) for value in (legacy, new, expectations, scope))
    legacy, new, expectations, scope = (value if isinstance(value, dict) else {}
                                        for value in (legacy, new, expectations, scope))
    try:
        errors = context_errors(legacy, new, expectations, scope)
    except (KeyError, TypeError, AttributeError):
        errors = ["comparison context missing or malformed"]
    if expectations.get("schema") != LABEL_SCHEMA:
        errors.append("independent density label schema missing or unsupported")
    expected_scope = expectations.get("scope")
    policy = expected_scope.get("policy") if isinstance(expected_scope, dict) else None
    error = policy_error(policy)
    cases = expectations.get("source_cases")
    malformed_cases = not isinstance(cases, list) or not cases
    if malformed_cases:
        cases = [None]
    names = [case.get("function") for case in cases if isinstance(case, dict)]
    rows = []
    for case in cases:
        name = case.get("function") if isinstance(case, dict) else None
        row = {"function": name, "rule_id": RULE}
        if malformed_inputs or malformed_cases or not isinstance(case, dict):
            finish(row, "uncovered", "comparison inputs or source case roster malformed or missing")
        elif errors:
            row.update(classification="context-mismatch", reasons=errors)
        elif error or not isinstance(name, str) or not name or names.count(name) != 1:
            finish(row, "uncovered", error or "independent function labels missing or duplicated")
        else:
            row = assess_case(legacy, new, case, scope, policy)
        rows.append(row)
    counts = dict(sorted(Counter(row["classification"] for row in rows).items()))
    successful = bool(rows) and all(row["classification"] in SUCCESSES for row in rows)
    return {"schema": "rstyle.clones-capability.density-comparison/1", "context": scope,
            "rows": rows, "counts": counts, "successful": successful,
            "fully_complete": successful and not counts.get("expected-uncovered", 0),
            "summary": {"cases": len(rows), "measured": sum(row["classification"] in ("evaluated-agreement", "intentional-difference") and nonnegative_integer(row.get("expected", {}).get("actual")) for row in rows),
                        "expected_uncovered": counts.get("expected-uncovered", 0), "policy_excluded": counts.get("policy-excluded-agreement", 0)},
            "limitations": ["Static policy events do not measure runtime frequency or semantic equivalence.",
                            "Legacy clean agreement checks a complete diagnostic roster, not an unavailable legacy numeric measurement.",
                            "Expected-uncovered cases remain outside the measured denominator and prevent full completion.",
                            "Scope sidecar is an operator declaration, not compiler evidence."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ("legacy", "new", "expectations", "scope", "output"):
        parser.add_argument("--" + option, required=True, type=Path)
    args = parser.parse_args()
    inputs = [json.loads(getattr(args, key).read_text()) for key in ("legacy", "new", "expectations", "scope")]
    inputs[2]["scope"]["manifest"] = str((args.expectations.parent / inputs[2]["scope"]["manifest"]).resolve())
    for mode, report in zip(("legacy", "new"), inputs[:2]):
        context = inputs[3].get(mode, {}).get("context", {})
        if "manifest" in context:
            context["manifest"] = str(Path(context["manifest"]).resolve())
        manifest = report.get("scope", {}).get("manifest_path")
        if manifest is not None:
            report["scope"]["manifest_path"] = str(Path(manifest).resolve())
    result = compare(*inputs)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    return int(not result["successful"])


if __name__ == "__main__":
    raise SystemExit(main())
