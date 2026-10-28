"""Independent spacing labels and policy cases; no producer captures required."""
import copy
import unittest

from compare import compare
from test_compare import fixture


def spacing_fixture(min_statements=3, statements=3, finding=True):
    inputs = fixture()
    for context in (inputs[2]["scope"], inputs[3]["legacy"]["context"],
                    inputs[3]["new"]["context"]):
        context["policy"]["min_statements"] = min_statements
    case = inputs[2]["source_cases"][0]
    case.update(spacing="finding" if finding else "evaluated-no-finding",
                top_statements=statements)
    assessment = {"rule_id": "fn_spacing", "outcome": "finding" if finding else "evaluated_no_finding",
                  "actual": statements, "limit": max(min_statements - 1, 0),
                  "availability": "known", "coverage": "complete"}
    inputs[1]["analysis"]["functions"][0]["assessments"].append(assessment)
    if finding:
        inputs[0]["diagnostics"].append({"rule_id": "fn_spacing", "path": "lib.rs", "line": 4,
                                            "actual": statements, "limit": assessment["limit"]})
    return inputs


def disabled_fixture():
    inputs = spacing_fixture(min_statements=0, finding=False)
    inputs[1]["analysis"]["functions"][0]["assessments"][-1].update(
        actual=None, availability="not_applicable")
    return inputs


class SpacingComparisonTests(unittest.TestCase):
    def spacing_row(self, inputs):
        rows = compare(*inputs)["rows"]
        self.assertEqual(len(rows), 3)
        return rows[-1]

    def assert_spacing(self, inputs, classification):
        row = self.spacing_row(inputs)
        self.assertEqual(row["classification"], classification)
        return row

    def test_explicit_finding_and_clean_labels_are_measured(self):
        for statements, finding in ((3, True), (1, False), (9, False)):
            with self.subTest(statements=statements, finding=finding):
                # A high count alone cannot establish missing blank lines.
                inputs = spacing_fixture(statements=statements, finding=finding)
                row = self.assert_spacing(inputs, "evaluated-agreement")
                self.assertEqual(row["new"]["actual"], statements)
                self.assertEqual(compare(*inputs)["counts"], {"evaluated-agreement": 3})

    def test_unlabeled_spacing_is_not_scored(self):
        inputs = spacing_fixture()
        inputs[2]["source_cases"][0].pop("spacing")
        self.assertEqual(compare(*inputs)["counts"], {"evaluated-agreement": 2})
        self.assertEqual(compare(*fixture())["counts"], {"evaluated-agreement": 2})

    def test_wrong_measure_limit_and_outcome_are_regressions(self):
        for mutation in ({"actual": 4}, {"limit": 3}, {"outcome": "evaluated_no_finding"}):
            with self.subTest(mutation=mutation):
                inputs = spacing_fixture()
                inputs[1]["analysis"]["functions"][0]["assessments"][-1].update(mutation)
                self.assert_spacing(inputs, "actual-regression")

    def test_missing_or_unsupported_assessment_has_no_sibling_authority(self):
        for mode in ("missing", "unsupported", "unknown", "incomplete", "null-actual"):
            with self.subTest(mode=mode):
                inputs = spacing_fixture()
                assessments = inputs[1]["analysis"]["functions"][0]["assessments"]
                if mode == "missing":
                    assessments.pop()
                else:
                    mutation = {"unsupported": {"availability": "unsupported", "actual": None},
                                "unknown": {"availability": "unknown"},
                                "incomplete": {"coverage": "incomplete"},
                                "null-actual": {"actual": None}}[mode]
                    assessments[-1].update(mutation)
                self.assert_spacing(inputs, "uncovered")

    def test_integer_label_and_measured_shapes_reject_booleans(self):
        invalid = (None, True, False, "3", -1, 3.0, [], {})
        for value in invalid:
            for field in ("actual", "limit", "top_statements"):
                with self.subTest(field=field, value=value):
                    inputs = spacing_fixture()
                    target = (inputs[2]["source_cases"][0] if field == "top_statements" else
                              inputs[1]["analysis"]["functions"][0]["assessments"][-1])
                    target[field] = value
                    self.assert_spacing(inputs, "uncovered")

    def test_missing_numeric_label_is_uncovered(self):
        inputs = spacing_fixture()
        inputs[2]["source_cases"][0].pop("top_statements")
        self.assert_spacing(inputs, "uncovered")

    def test_missing_or_invalid_spacing_policy_is_uncovered_without_crashing(self):
        for value in (None, True, False, "3", -1, 3.0, [], {}):
            with self.subTest(value=value):
                inputs = spacing_fixture()
                inputs[2]["scope"]["policy"]["min_statements"] = value
                self.assert_spacing(inputs, "uncovered")
        inputs = spacing_fixture()
        inputs[2]["scope"]["policy"].pop("min_statements")
        self.assert_spacing(inputs, "uncovered")

    def test_spacing_sidecar_policy_mismatch_is_checked_first(self):
        for side in ("new", "legacy"):
            inputs = spacing_fixture()
            # Context policy dictionaries must be independently owned here.
            context = inputs[3][side]["context"]
            context["policy"] = dict(context["policy"], min_statements=4)
            self.assertEqual(compare(*inputs)["counts"], {"context-mismatch": 3})

    def test_duplicate_and_budget_rows_stay_uncovered(self):
        for mode in ("duplicate-rule", "duplicate-function", "budget", "legacy-duplicate"):
            with self.subTest(mode=mode):
                inputs = spacing_fixture()
                analysis = inputs[1]["analysis"]
                if mode == "duplicate-rule":
                    assessments = analysis["functions"][0]["assessments"]
                    assessments.append(copy.deepcopy(assessments[-1]))
                elif mode == "duplicate-function":
                    analysis["functions"].append(copy.deepcopy(analysis["functions"][0]))
                elif mode == "budget":
                    analysis["execution"] = "budget_exhausted"
                else:
                    functions = inputs[3]["legacy"]["functions"]
                    functions.append(copy.deepcopy(functions[0]))
                self.assert_spacing(inputs, "uncovered")

    def test_spacing_suppression_permission_is_independent_of_raw_measure(self):
        inputs = spacing_fixture()
        inputs[2]["source_cases"][0]["suppressed_rules"] = ["fn_spacing"]
        inputs[0]["diagnostics"].pop()
        function = inputs[1]["analysis"]["functions"][0]
        function["source_suppression"] = {"status": "allowed", "rules": ["fn_spacing"]}
        row = self.assert_spacing(inputs, "evaluated-agreement")
        self.assertEqual(row["new"]["outcome"], "finding")
        self.assertEqual(row["new_visible_outcome"], "evaluated_no_finding")
        function["assessments"][-1]["actual"] = 8
        self.assert_spacing(inputs, "actual-regression")
        function["assessments"][-1]["actual"] = 3
        function["source_suppression"]["rules"] = ["fn_length"]
        self.assert_spacing(inputs, "actual-regression")

    def test_exclusion_skips_labeled_spacing_without_claiming_measurement(self):
        inputs = spacing_fixture()
        inputs[0]["diagnostics"] = []
        inputs[2]["source_cases"][0]["excluded"] = True
        inputs[3]["legacy"]["functions"][0]["excluded"] = True
        function = inputs[1]["analysis"]["functions"][0]
        function["source_exclusion"] = {"status": "excluded", "path": "src/excluded.rs"}
        for assessment in function["assessments"]:
            assessment.update(outcome="not_evaluated", coverage="complete",
                              availability="not_applicable", actual=None)
        self.assert_spacing(inputs, "policy-excluded-agreement")
        function["assessments"][-1]["limit"] = 9
        self.assert_spacing(inputs, "actual-regression")

    def test_disabled_policy_agreement_is_not_measured_clean(self):
        row = self.assert_spacing(disabled_fixture(), "policy-disabled-agreement")
        self.assertIsNone(row["new"]["actual"])

    def test_disabled_wrong_limit_and_outcome_are_regressions(self):
        for mutation in ({"limit": 1}, {"outcome": "finding"}, {"outcome": "not_evaluated"}):
            with self.subTest(mutation=mutation):
                inputs = disabled_fixture()
                inputs[1]["analysis"]["functions"][0]["assessments"][-1].update(mutation)
                self.assert_spacing(inputs, "actual-regression")

    def test_disabled_malformed_availability_coverage_and_actual_are_uncovered(self):
        for mutation in ({"availability": "known"}, {"coverage": "incomplete"},
                         {"actual": 0}, {"actual": False}, {"limit": False}):
            with self.subTest(mutation=mutation):
                inputs = disabled_fixture()
                inputs[1]["analysis"]["functions"][0]["assessments"][-1].update(mutation)
                self.assert_spacing(inputs, "uncovered")
        for field in ("actual", "limit", "availability", "coverage", "outcome"):
            inputs = disabled_fixture()
            inputs[1]["analysis"]["functions"][0]["assessments"][-1].pop(field)
            self.assert_spacing(inputs, "uncovered")

    def test_disabled_requires_complete_clean_legacy_roster(self):
        for mode in ("missing", "incomplete", "finding"):
            with self.subTest(mode=mode):
                inputs = disabled_fixture()
                if mode == "missing":
                    inputs[3]["legacy"]["functions"] = []
                elif mode == "incomplete":
                    inputs[3]["legacy"]["complete"] = False
                else:
                    inputs[0]["diagnostics"].append({"rule_id": "fn_spacing", "path": "lib.rs",
                                                       "line": 4, "actual": 3, "limit": 0})
                self.assert_spacing(inputs, "uncovered")

    def test_disabled_still_checks_suppression_permission(self):
        inputs = disabled_fixture()
        inputs[2]["source_cases"][0]["suppressed_rules"] = ["fn_spacing"]
        self.assert_spacing(inputs, "actual-regression")
        inputs[1]["analysis"]["functions"][0]["source_suppression"] = {
            "status": "allowed", "rules": ["fn_spacing"]}
        self.assert_spacing(inputs, "policy-disabled-agreement")
        inputs[1]["analysis"]["functions"][0]["source_suppression"] = {"status": "unavailable"}
        self.assert_spacing(inputs, "uncovered")

    def test_disabled_cannot_override_expected_exclusion(self):
        inputs = disabled_fixture()
        inputs[2]["source_cases"][0]["excluded"] = True
        inputs[3]["legacy"]["functions"][0]["excluded"] = True
        inputs[0]["diagnostics"] = []
        self.assert_spacing(inputs, "actual-regression")


if __name__ == "__main__":
    unittest.main()
