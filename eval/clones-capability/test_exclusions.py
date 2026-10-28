"""Independent exclusion-policy acceptance cases, without producer captures."""
import copy
import unittest

from compare import compare
from test_compare import fixture


def excluded_fixture():
    inputs = fixture()
    inputs[0]["diagnostics"] = []
    inputs[2]["source_cases"][0]["excluded"] = True
    inputs[3]["legacy"]["functions"][0]["excluded"] = True
    function = inputs[1]["analysis"]["functions"][0]
    function["source_exclusion"] = {"status": "excluded", "path": "src/excluded.rs"}
    for assessment in function["assessments"]:
        assessment.update(outcome="not_evaluated", coverage="complete",
                          availability="not_applicable", actual=None)
    return inputs


class ExclusionComparisonTests(unittest.TestCase):
    def assert_classifications(self, inputs, expected):
        result = compare(*inputs)
        self.assertEqual(result["counts"], {expected: 2})
        return result

    def test_excluded_agreement_is_distinct_from_measured_agreement(self):
        result = self.assert_classifications(excluded_fixture(), "policy-excluded-agreement")
        for row in result["rows"]:
            self.assertIsNone(row["new"]["actual"])
            self.assertNotEqual(row["classification"], "evaluated-agreement")
        # Independent numeric labels remain irrelevant to an intentional skip.
        inputs = excluded_fixture()
        inputs[2]["source_cases"][0].update(code_lines=987, parameters=654)
        self.assert_classifications(inputs, "policy-excluded-agreement")

    def test_historical_absence_and_explicit_inclusion_preserve_comparison(self):
        self.assert_classifications(fixture(), "evaluated-agreement")
        for status in ("included", "not_applicable"):
            with self.subTest(status=status):
                inputs = fixture()
                inputs[1]["analysis"]["functions"][0]["source_exclusion"] = {
                    "status": status, "path": "src/kept.rs"}
                inputs[2]["source_cases"][0]["excluded"] = False
                inputs[3]["legacy"]["functions"][0]["excluded"] = False
                self.assert_classifications(inputs, "evaluated-agreement")

    def test_known_missing_exclusion_is_regression(self):
        for declaration in ("absent", {"status": "included", "path": "src/kept.rs"},
                            {"status": "not_applicable", "path": "/other/lib.rs"}):
            with self.subTest(declaration=declaration):
                inputs = fixture()
                inputs[2]["source_cases"][0]["excluded"] = True
                inputs[3]["legacy"]["functions"][0]["excluded"] = True
                inputs[0]["diagnostics"] = []
                if declaration != "absent":
                    inputs[1]["analysis"]["functions"][0]["source_exclusion"] = declaration
                self.assert_classifications(inputs, "actual-regression")

    def test_unexpected_new_or_legacy_exclusion_is_regression(self):
        inputs = excluded_fixture()
        inputs[2]["source_cases"][0]["excluded"] = False
        self.assert_classifications(inputs, "actual-regression")
        inputs = fixture()
        inputs[3]["legacy"]["functions"][0]["excluded"] = True
        inputs[0]["diagnostics"] = []
        self.assert_classifications(inputs, "actual-regression")

    def test_missing_legacy_exclusion_declaration_is_regression(self):
        for value in ("absent", False):
            with self.subTest(value=value):
                inputs = excluded_fixture()
                entry = inputs[3]["legacy"]["functions"][0]
                entry.pop("excluded")
                if value is False:
                    entry["excluded"] = False
                self.assert_classifications(inputs, "actual-regression")

    def test_unavailable_and_malformed_exclusion_are_uncovered(self):
        declarations = (None, [], "excluded", {}, {"status": "future"},
                        {"status": "unavailable", "reason": "source missing"})
        for declaration in declarations:
            for factory in (fixture, excluded_fixture):
                with self.subTest(declaration=declaration, fixture=factory.__name__):
                    inputs = factory()
                    inputs[1]["analysis"]["functions"][0]["source_exclusion"] = declaration
                    self.assert_classifications(inputs, "uncovered")

    def test_present_known_exclusion_requires_nonempty_string_path(self):
        for status in ("included", "excluded", "not_applicable"):
            for path in (None, "", 12, [], False):
                with self.subTest(status=status, path=path):
                    inputs = excluded_fixture() if status == "excluded" else fixture()
                    declaration = {"status": status}
                    if path is not None:
                        declaration["path"] = path
                    inputs[1]["analysis"]["functions"][0]["source_exclusion"] = declaration
                    self.assert_classifications(inputs, "uncovered")

    def test_excluded_skip_requires_exact_complete_not_applicable_shape(self):
        mutations = ({"outcome": "finding"}, {"outcome": "evaluated_no_finding"},
                     {"availability": "known"}, {"availability": "unknown"},
                     {"coverage": "incomplete"}, {"actual": 0}, {"limit": None},
                     {"limit": "5"}, {"limit": True}, {"limit": -1})
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                inputs = excluded_fixture()
                assessment = inputs[1]["analysis"]["functions"][0]["assessments"][0]
                assessment.update(mutation)
                result = compare(*inputs)
                self.assertEqual(result["rows"][0]["classification"], "uncovered")
                self.assertEqual(result["rows"][1]["classification"], "policy-excluded-agreement")

    def test_excluded_skip_wrong_configured_limit_is_policy_regression(self):
        inputs = excluded_fixture()
        inputs[1]["analysis"]["functions"][0]["assessments"][0]["limit"] = 100
        result = compare(*inputs)
        self.assertEqual(result["rows"][0]["classification"], "actual-regression")
        self.assertEqual(result["rows"][1]["classification"], "policy-excluded-agreement")

    def test_exclusion_labels_and_legacy_declarations_require_booleans(self):
        for value in (None, 0, 1, "true", [], {}):
            for location in ("label", "legacy"):
                with self.subTest(value=value, location=location):
                    inputs = excluded_fixture()
                    if location == "label":
                        inputs[2]["source_cases"][0]["excluded"] = value
                    else:
                        inputs[3]["legacy"]["functions"][0]["excluded"] = value
                    self.assert_classifications(inputs, "uncovered")

    def test_excluded_skip_requires_actual_and_limit_fields(self):
        for field in ("actual", "limit", "outcome", "coverage", "availability"):
            with self.subTest(field=field):
                inputs = excluded_fixture()
                inputs[1]["analysis"]["functions"][0]["assessments"][0].pop(field)
                self.assertEqual(compare(*inputs)["rows"][0]["classification"], "uncovered")

    def test_excluded_legacy_requires_complete_unique_roster(self):
        for mode in ("missing", "duplicate", "incomplete"):
            with self.subTest(mode=mode):
                inputs = excluded_fixture()
                legacy = inputs[3]["legacy"]
                if mode == "missing":
                    legacy["functions"] = []
                elif mode == "duplicate":
                    legacy["functions"].append(copy.deepcopy(legacy["functions"][0]))
                else:
                    legacy["complete"] = False
                self.assert_classifications(inputs, "uncovered")

    def test_excluded_legacy_cannot_have_numeric_diagnostic(self):
        inputs = excluded_fixture()
        inputs[0]["diagnostics"] = fixture()[0]["diagnostics"]
        result = compare(*inputs)
        self.assertEqual(result["rows"][0]["classification"], "uncovered")
        self.assertEqual(result["rows"][1]["classification"], "policy-excluded-agreement")

    def test_exclusions_do_not_bypass_failed_execution_or_duplicate_rows(self):
        for mode in ("failed", "budget", "function-failed", "duplicate-function", "duplicate-rule"):
            with self.subTest(mode=mode):
                inputs = excluded_fixture()
                analysis = inputs[1]["analysis"]
                function = analysis["functions"][0]
                if mode in ("failed", "budget"):
                    analysis["execution"] = "failed" if mode == "failed" else "budget_exhausted"
                elif mode == "function-failed":
                    function["execution"] = "failed"
                elif mode == "duplicate-function":
                    analysis["functions"].append(copy.deepcopy(function))
                else:
                    function["assessments"].extend(copy.deepcopy(function["assessments"]))
                self.assert_classifications(inputs, "uncovered")

    def test_aggregate_exclusion_counts_do_not_authorize_skips(self):
        inputs = excluded_fixture()
        inputs[2]["source_cases"][0].pop("excluded")
        inputs[1]["analysis"]["scope"].update(excluded_source_functions=1, excluded_rules=2)
        self.assert_classifications(inputs, "actual-regression")


if __name__ == "__main__":
    unittest.main()
