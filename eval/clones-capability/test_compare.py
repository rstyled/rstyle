"""Independent report-shape tests; no compiler capture or producer output used."""
import copy
import unittest

from compare import compare


def fixture():
    context = {"manifest": "corpus/Cargo.toml", "package": "sample", "target": "lib", "features": [],
               "profile": "dev", "toolchain": "pinned", "revision": "fixture", "policy":
               {"max_lines": 5, "max_params": 4, "count_self": False}}
    labels = {"scope": context.copy(), "source_cases": [{"function": "rules::over", "code_lines": 6,
              "parameters": 1, "length": "finding", "parameter_count": "evaluated-no-finding"}]}
    metadata = {"legacy": {"context": context.copy(), "complete": True,
                "functions": [{"name": "sample::rules::over", "source_file": "lib.rs", "line": 4}]},
                "new": {"context": context.copy()}}
    legacy = {"diagnostics": [{"rule_id": "fn_length", "path": "lib.rs", "line": 4,
                              "actual": 6, "limit": 5}]}
    new = {"analysis": {"schema": "rstyle.consumer-analysis/1", "execution": "completed",
           "scope": {"coverage": "complete", "assessed_functions": 1, "expected_functions": 1},
           "functions": [{"name": "sample::rules::over", "execution": "completed", "assessments": [
               {"rule_id": "fn_length", "outcome": "finding", "actual": 6, "limit": 5,
                "availability": "known", "coverage": "complete"},
               {"rule_id": "fn_params", "outcome": "evaluated_no_finding", "actual": 1, "limit": 4,
                "availability": "known", "coverage": "complete"}]}]}}
    return legacy, new, labels, metadata


class ComparisonTests(unittest.TestCase):
    def test_agreement_including_explicit_clean_assessment(self):
        self.assertEqual(compare(*fixture())["counts"], {"evaluated-agreement": 2})

    def test_missing_new_assessment_is_uncovered(self):
        inputs = fixture()
        inputs[1]["analysis"]["functions"][0]["assessments"].pop()
        self.assertEqual(compare(*inputs)["rows"][1]["classification"], "uncovered")

    def test_unknown_does_not_become_clean(self):
        inputs = fixture()
        inputs[1]["analysis"]["functions"][0]["assessments"][1]["availability"] = "unknown"
        self.assertEqual(compare(*inputs)["rows"][1]["classification"], "uncovered")

    def test_wrong_measure_is_regression_even_if_outcome_matches(self):
        inputs = fixture()
        inputs[1]["analysis"]["functions"][0]["assessments"][0]["actual"] = 7
        self.assertEqual(compare(*inputs)["rows"][0]["classification"], "actual-regression")

    def test_clean_absence_requires_complete_legacy_roster(self):
        inputs = fixture()
        inputs[3]["legacy"]["complete"] = False
        self.assertEqual(compare(*inputs)["rows"][1]["classification"], "uncovered")

    def test_duplicate_contexts_are_not_merged(self):
        inputs = fixture()
        functions = inputs[1]["analysis"]["functions"]
        functions.append(copy.deepcopy(functions[0]))
        self.assertEqual(compare(*inputs)["counts"], {"uncovered": 2})

    def test_context_mismatch_and_missing_context(self):
        for key in ("profile", "manifest", "policy"):
            inputs = fixture()
            inputs[3]["new"]["context"].pop(key)
            self.assertEqual(compare(*inputs)["counts"], {"context-mismatch": 2})

    def test_report_cannot_contradict_scope_sidecar(self):
        inputs = fixture()
        inputs[1]["scope"] = {"manifest_path": "other/Cargo.toml"}
        self.assertEqual(compare(*inputs)["counts"], {"context-mismatch": 2})

    def test_budget_failure_uncovered_not_success(self):
        inputs = fixture()
        inputs[1]["analysis"]["execution"] = "budget_exhausted"
        self.assertEqual(compare(*inputs)["counts"], {"uncovered": 2})

    def test_improvement_requires_reviewed_label(self):
        inputs = fixture()
        inputs[0]["diagnostics"] = []
        self.assertEqual(compare(*inputs)["rows"][0]["classification"], "unreviewed-disagreement")
        inputs[2]["source_cases"][0]["reviewed_improvements"] = {"fn_length": "Reviewed independent length count."}
        self.assertEqual(compare(*inputs)["rows"][0]["classification"], "intentional-improvement")

    def test_missing_legacy_function_cannot_be_clean(self):
        inputs = fixture()
        inputs[3]["legacy"]["functions"] = []
        self.assertEqual(compare(*inputs)["counts"], {"uncovered": 2})

    def test_pair_and_abstraction_labels_not_scored_as_source_rules(self):
        inputs = fixture()
        inputs[2]["strict_pair_cases"] = [{"expected": "accepted"}]
        inputs[2]["abstraction_cases"] = [{"expected": "no-proposal"}]
        self.assertEqual(len(compare(*inputs)["rows"]), 2)


if __name__ == "__main__":
    unittest.main()
