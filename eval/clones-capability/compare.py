#!/usr/bin/env python3
"""Compare explicit legacy/new source-rule reports against independent labels."""
import argparse
import json
from pathlib import Path

RULES = {"length": "fn_length", "parameter_count": "fn_params"}
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
    if hits:
        return {"outcome": "finding", "actual": hits[0].get("actual"), "limit": hits[0].get("limit")}, None
    if metadata.get("complete") is not True:
        return None, "legacy assessment completeness unavailable"
    return {"outcome": "evaluated_no_finding"}, None


def new_assessment(report, name, rule):
    analysis = report.get("analysis", {})
    if analysis.get("schema") != "rstyle.consumer-analysis/1":
        return None, None, "new analysis schema missing or unsupported"
    if analysis.get("execution") != "completed":
        return None, None, "new execution not completed"
    functions = [entry for entry in analysis.get("functions", []) if named(entry["name"], name)]
    if len(functions) != 1:
        return None, None, "new function missing or ambiguous across contexts"
    function = functions[0]
    if function.get("execution") != "completed":
        return None, None, "function execution not completed"
    assessments = [entry for entry in function.get("assessments", []) if entry.get("rule_id") == rule]
    if len(assessments) != 1:
        return None, None, "new rule assessment missing or ambiguous"
    assessment = assessments[0]
    if (assessment.get("outcome") == "not_evaluated"
            or assessment.get("availability") != "known"
            or assessment.get("coverage") != "complete"):
        return None, None, "new rule has unavailable or incomplete evidence"
    if assessment.get("outcome") not in ("finding", "evaluated_no_finding"):
        return None, None, "unknown new assessment outcome"
    suppression, error = source_suppression(function, rule)
    return assessment, suppression, error


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


def compare(legacy, new, expectations, scope):
    errors = context_errors(legacy, new, expectations, scope)
    rows = []
    for case in expectations["source_cases"]:
        for label, rule in RULES.items():
            row = {"function": case["function"], "rule_id": rule, "expected": normalized(case[label])}
            if errors:
                row.update(classification="context-mismatch", reasons=errors)
                rows.append(row)
                continue
            old, old_error = legacy_assessment(legacy, scope["legacy"], case["function"], rule)
            current, suppression, current_error = new_assessment(new, case["function"], rule)
            # Labels describe permission; aggregate hidden-finding counts are not labels.
            row.update(legacy=old, new=current, source_suppression=suppression,
                       expected_suppressed=rule in case.get("suppressed_rules", []))
            if old_error or current_error:
                row.update(classification="uncovered", reasons=[reason for reason in (old_error, current_error) if reason])
            else:
                visible_outcome = ("evaluated_no_finding" if suppression["permitted"]
                                   else current["outcome"])
                row["new_visible_outcome"] = visible_outcome
                expected_actual = case["code_lines" if label == "length" else "parameters"]
                expected_limit = expectations["scope"]["policy"]["max_lines" if label == "length" else "max_params"]
                correct = (current["outcome"] == row["expected"]
                           and current.get("actual") == expected_actual
                           and current.get("limit") == expected_limit)
                if not correct:
                    row.update(classification="actual-regression", reasons=["new assessment contradicts independent label/count/policy"])
                elif suppression["permitted"] != row["expected_suppressed"]:
                    row.update(classification="actual-regression", reasons=["new suppression contradicts independent suppression label"])
                elif (old["outcome"] == visible_outcome
                      and (old["outcome"] != "finding"
                           or (old.get("actual") == expected_actual and old.get("limit") == expected_limit))):
                    row.update(classification="evaluated-agreement", reasons=[])
                elif case.get("reviewed_improvements", {}).get(rule):
                    row.update(classification="intentional-improvement", reasons=[case["reviewed_improvements"][rule]])
                else:
                    row.update(classification="unreviewed-disagreement", reasons=["new matches labels; legacy disagreement requires explicit review"])
            rows.append(row)
    counts = {key: sum(row["classification"] == key for row in rows) for key in sorted({row["classification"] for row in rows})}
    return {"schema": "rstyle.clones-capability.comparison/1", "context": scope, "counts": counts,
            "rows": rows, "limitations": ["Source/signature rules only; strict pairs do not score abstraction proposals.",
                                          "Invalid-suppression diagnostics are outside the two measured source-rule comparisons.",
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
    return int(any(row["classification"] not in ("evaluated-agreement", "intentional-improvement") for row in result["rows"]))


if __name__ == "__main__":
    raise SystemExit(main())
