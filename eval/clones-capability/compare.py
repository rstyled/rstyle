#!/usr/bin/env python3
"""Compare explicit legacy/new source-rule reports against independent labels."""
import argparse
import json
from pathlib import Path

RULES = {"length": "fn_length", "parameter_count": "fn_params", "spacing": "fn_spacing"}
MEASURES = {"length": "code_lines", "parameter_count": "parameters", "spacing": "top_statements"}
POLICIES = {"length": "max_lines", "parameter_count": "max_params", "spacing": "min_statements"}
CONTEXT_FIELDS = ("manifest", "package", "target", "features", "profile", "toolchain", "revision", "policy")


def normalized(value):
    return value.replace("-", "_")


def named(name, expected):
    return name == expected or name.endswith("::" + expected)


def context_errors(legacy, new, expected, scope):
    errors = []
    contexts = [scope.get(mode, {}).get("context", {}) for mode in ("legacy", "new")]
    for key in CONTEXT_FIELDS:
        if any(key not in context for context in contexts):
            errors.append("missing comparison context: " + key)
        elif contexts[0][key] != contexts[1][key]:
            errors.append("different comparison context: " + key)
    for index, report in enumerate((legacy, new)):
        context = contexts[index]
        for key in ("manifest", "package", "target", "features", "policy"):
            if context.get(key) != expected["scope"][key]:
                errors.append(("legacy", "new")[index] + " differs from labels: " + key)
        report_manifest = report.get("scope", {}).get("manifest_path")
        if report_manifest is not None and report_manifest != context.get("manifest"):
            errors.append("report contradicts declared manifest")
        if "toolchain" in report and report["toolchain"] != context.get("toolchain"):
            errors.append("report contradicts declared toolchain")
    return sorted(set(errors))


def legacy_assessment(report, metadata, name, rule):
    roster = [entry for entry in metadata.get("functions", []) if named(entry["name"], name)]
    if len(roster) != 1:
        return None, "legacy function roster missing or ambiguous"
    entry = roster[0]
    diagnostics = report.get("diagnostics", report.get("violations", []))
    if not isinstance(diagnostics, list):
        return None, "legacy diagnostic list unavailable"
    hits = [diagnostic for diagnostic in diagnostics
            if diagnostic.get("rule_id") == rule
            and diagnostic.get("path") == entry["source_file"]
            and diagnostic.get("line") == entry["line"]]
    if len(hits) > 1:
        return None, "legacy diagnostic duplicated across contexts"
    excluded = entry.get("excluded", False)
    if not isinstance(excluded, bool):
        return None, "legacy exclusion declaration malformed"
    if excluded:
        if metadata.get("complete") is not True:
            return None, "legacy exclusion completeness unavailable"
        if hits:
            return None, "legacy diagnostic contradicts declared exclusion"
        return {"outcome": "not_evaluated", "excluded": True}, None
    if hits:
        return {"outcome": "finding", "actual": hits[0].get("actual"), "limit": hits[0].get("limit")}, None
    if metadata.get("complete") is not True:
        return None, "legacy assessment completeness unavailable"
    return {"outcome": "evaluated_no_finding"}, None


def source_exclusion(function):
    """Check the explicit policy decision; never infer it from missing findings."""
    if "source_exclusion" not in function:
        return {"status": "absent", "excluded": False}, None
    declaration = function["source_exclusion"]
    if not isinstance(declaration, dict):
        return None, "new exclusion observation malformed"
    status = declaration.get("status")
    if status == "unavailable":
        return None, "new exclusion observation unavailable"
    if status not in ("included", "excluded", "not_applicable"):
        return None, "new exclusion observation has unknown status"
    path = declaration.get("path")
    if not isinstance(path, str) or not path:
        return None, "new exclusion path missing or malformed"
    return {"status": status, "excluded": status == "excluded", "path": path}, None


def new_assessment(report, name, rule, disabled=False):
    analysis = report.get("analysis", {})
    if analysis.get("schema") != "rstyle.consumer-analysis/1":
        return None, None, None, "new analysis schema missing or unsupported"
    if analysis.get("execution") != "completed":
        return None, None, None, "new execution not completed"
    functions = [entry for entry in analysis.get("functions", []) if named(entry["name"], name)]
    if len(functions) != 1:
        return None, None, None, "new function missing or ambiguous across contexts"
    function = functions[0]
    if function.get("execution") != "completed":
        return None, None, None, "function execution not completed"
    assessments = [entry for entry in function.get("assessments", []) if entry.get("rule_id") == rule]
    if len(assessments) != 1:
        return None, None, None, "new rule assessment missing or ambiguous"
    assessment = assessments[0]
    exclusion, error = source_exclusion(function)
    if error:
        return assessment, None, exclusion, error
    if exclusion["excluded"]:
        if (assessment.get("outcome") != "not_evaluated"
                or assessment.get("availability") != "not_applicable"
                or assessment.get("coverage") != "complete"
                or "actual" not in assessment or assessment["actual"] is not None
                or type(assessment.get("limit")) is not int or assessment["limit"] < 0):
            return assessment, None, exclusion, "new excluded assessment contradicts complete policy skip"
        return assessment, None, exclusion, None
    if disabled:
        if (assessment.get("availability") != "not_applicable"
                or assessment.get("coverage") != "complete"
                or "actual" not in assessment or assessment["actual"] is not None
                or not nonnegative_integer(assessment.get("limit"))
                or "outcome" not in assessment):
            return assessment, None, exclusion, "new disabled assessment has unavailable or malformed policy evidence"
        suppression, error = source_suppression(function, rule)
        return assessment, suppression, exclusion, error
    if (assessment.get("outcome") == "not_evaluated"
            or assessment.get("availability") != "known"
            or assessment.get("coverage") != "complete"):
        return None, None, exclusion, "new rule has unavailable or incomplete evidence"
    if assessment.get("outcome") not in ("finding", "evaluated_no_finding"):
        return None, None, exclusion, "unknown new assessment outcome"
    suppression, error = source_suppression(function, rule)
    return assessment, suppression, exclusion, error


def source_suppression(function, rule):
    """Match suppression permission, independently of whether a finding exists."""
    if "source_suppression" not in function:
        return {"status": "absent", "permitted": False}, None
    declaration = function["source_suppression"]
    if not isinstance(declaration, dict):
        return None, "new suppression observation malformed"
    status = declaration.get("status")
    if status == "unavailable":
        return None, "new suppression observation unavailable"
    if status not in ("none", "allowed", "invalid"):
        return None, "new suppression observation has unknown status"
    rules = declaration.get("rules") if status == "allowed" else []
    if status == "allowed" and (not isinstance(rules, list)
                                or not all(isinstance(name, str) for name in rules)):
        return None, "new suppression rule roster malformed"
    return {"status": status, "permitted": status == "allowed" and rule in RULES.values()
            and rule in rules}, None


def nonnegative_integer(value):
    return type(value) is int and value >= 0


def rule_policy(expectations, label):
    value = expectations["scope"]["policy"].get(POLICIES[label])
    if not nonnegative_integer(value):
        return None, False, "independent rule policy missing or malformed"
    limit = max(value - 1, 0) if label == "spacing" else value
    return limit, label == "spacing" and value == 0, None


def measured_comparison(row, case, current, suppression, old, label, limit):
    actual = case.get(MEASURES[label])
    if not all(nonnegative_integer(value) for value in (actual, current.get("actual"), current.get("limit"))):
        row.update(classification="uncovered", reasons=["numeric measurement or independent count missing or malformed"])
        return
    visible = "evaluated_no_finding" if suppression["permitted"] else current["outcome"]
    row["new_visible_outcome"] = visible
    if (current["outcome"] != row["expected"] or current["actual"] != actual or current["limit"] != limit):
        row.update(classification="actual-regression", reasons=["new assessment contradicts independent label/count/policy"])
    elif suppression["permitted"] != row["expected_suppressed"]:
        row.update(classification="actual-regression", reasons=["new suppression contradicts independent suppression label"])
    elif (old["outcome"] == visible and (old["outcome"] != "finding"
          or (nonnegative_integer(old.get("actual")) and nonnegative_integer(old.get("limit"))
              and old["actual"] == actual and old["limit"] == limit))):
        row.update(classification="evaluated-agreement", reasons=[])
    elif case.get("reviewed_improvements", {}).get(row["rule_id"]):
        row.update(classification="intentional-improvement", reasons=[case["reviewed_improvements"][row["rule_id"]]])
    else:
        row.update(classification="unreviewed-disagreement", reasons=["new matches labels; legacy disagreement requires explicit review"])


def disabled_comparison(row, current, suppression, old):
    row["new_visible_outcome"] = "evaluated_no_finding"
    if (current["outcome"] != "evaluated_no_finding" or current["limit"] != 0
            or row["expected"] != "evaluated_no_finding"):
        row.update(classification="actual-regression", reasons=["disabled rule contradicts independent policy"])
    elif suppression["permitted"] != row["expected_suppressed"]:
        row.update(classification="actual-regression", reasons=["new suppression contradicts independent suppression label"])
    elif old["outcome"] != "evaluated_no_finding":
        row.update(classification="uncovered", reasons=["disabled legacy rule must have complete clean assessment"])
    else:
        row.update(classification="policy-disabled-agreement", reasons=[])


def compare(legacy, new, expectations, scope):
    errors = context_errors(legacy, new, expectations, scope)
    rows = []
    for case in expectations["source_cases"]:
        for label, rule in RULES.items():
            if label == "spacing" and label not in case:
                continue
            limit, disabled, policy_error = rule_policy(expectations, label)
            row = {"function": case["function"], "rule_id": rule, "expected": normalized(case[label])}
            if errors:
                row.update(classification="context-mismatch", reasons=errors)
                rows.append(row)
                continue
            old, old_error = legacy_assessment(legacy, scope["legacy"], case["function"], rule)
            current, suppression, exclusion, current_error = new_assessment(new, case["function"], rule, disabled)
            # Labels describe permission; aggregate hidden-finding counts are not labels.
            row.update(legacy=old, new=current, source_suppression=suppression,
                       source_exclusion=exclusion, expected_excluded=case.get("excluded", False),
                       expected_suppressed=rule in case.get("suppressed_rules", []))
            if old_error or current_error or policy_error:
                row.update(classification="uncovered", reasons=[reason for reason in (old_error, current_error, policy_error) if reason])
            elif not isinstance(row["expected_excluded"], bool):
                row.update(classification="uncovered", reasons=["independent exclusion label malformed"])
            elif (old.get("excluded", False) != row["expected_excluded"]
                  or exclusion["excluded"] != row["expected_excluded"]):
                row.update(classification="actual-regression", reasons=["exclusion contradicts independent policy label"])
            elif row["expected_excluded"]:
                if current.get("limit") != limit:
                    row.update(classification="actual-regression", reasons=["excluded rule limit contradicts independent policy"])
                else:
                    row.update(classification="policy-excluded-agreement", reasons=[])
            elif disabled:
                disabled_comparison(row, current, suppression, old)
            else:
                measured_comparison(row, case, current, suppression, old, label, limit)
            rows.append(row)
    counts = {key: sum(row["classification"] == key for row in rows) for key in sorted({row["classification"] for row in rows})}
    return {"schema": "rstyle.clones-capability.comparison/1", "context": scope, "counts": counts,
            "rows": rows, "limitations": ["Source/signature rules only; strict pairs do not score abstraction proposals.",
                                          "Invalid-suppression diagnostics are outside the measured source-rule comparisons.",
                                          "Policy-excluded and policy-disabled agreements check policy results, not source measurements.",
                                          "Scope sidecar is an explicit operator declaration, not compiler evidence."]}


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
    return int(any(row["classification"] not in ("evaluated-agreement", "intentional-improvement", "policy-excluded-agreement", "policy-disabled-agreement") for row in result["rows"]))


if __name__ == "__main__":
    raise SystemExit(main())
