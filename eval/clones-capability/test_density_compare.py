"""Independent numeric fixtures; no compiler or generated labels required."""
import copy
import unittest

from density_compare import LABEL_SCHEMA, compare


def fixture():
    policy = {"max_density_milli": 500, "min_statements": 0, "min_lines": 0, "transitive_depth": 2,
              "weights": dict(mutation=1, io=3, network=3, global_=2, unsafe=4, time=1, random=1, panic=0, unknown=1),
              "paths": dict(io=[], network=[], time=[], random=[], panic=[]), "exclude": []}
    policy["weights"]["global"] = policy["weights"].pop("global_")
    context = dict(manifest="corpus/Cargo.toml", package="sample", target="lib", features=[], profile="dev", toolchain="pinned", revision="fixture", policy=policy)
    labels = {"schema": LABEL_SCHEMA, "scope": copy.deepcopy(context), "source_cases": [{"function": "over", "expected": dict(events={"mutation": 1}, statements=1, code_lines=3, actual=1000, outcome="finding", complete=True)}]}
    metadata = {"legacy": {"context": copy.deepcopy(context), "complete": True, "functions": [{"name": "sample::over", "source_file": "lib.rs", "line": 4}]}, "new": {"context": copy.deepcopy(context)}}
    legacy = {"diagnostics": [{"rule_id": "side_effect_density", "path": "lib.rs", "line": 4, "actual": 1000, "limit": 500}]}
    density = {"complete": True, "visible_findings": 1, "findings": 1, "event_inventory": {"coverage": "complete", "counts": {"mutation": 1}, "events": [{"kind": "mutation"}], "unresolved_boundaries": []},
               "measurement": {"source": {"statements": 1, "code_lines": 3}, "actual": 1000, "limit": 500, "exempt": False, "outcome": "finding"},
               "source_suppression": {"status": "none"}, "source_exclusion": {"status": "included", "path": "src/lib.rs"}, "suppressed": False, "invalid_suppression": None}
    side_effects = {key: copy.deepcopy(value) for key, value in policy.items() if key != "exclude"}
    new = {"capture_complete": True, "assessment_complete": True, "mutation_effects": {"schema": "rstyle.mutation-effects/4", "density_requested": True, "execution": "completed", "density_metric": "rstyle.static-effect-events/1", "statement_metric": "rstyle.original-source-statements/1", "density_config": {"exclude": [], "side_effects": side_effects}, "functions": [{"name": "sample::over", "evidence": {"execution": "completed"}, "density": density}]}}
    return legacy, new, labels, metadata


def density(inputs):
    return inputs[1]["mutation_effects"]["functions"][0]["density"]


def classification(inputs):
    return compare(*inputs)["rows"][0]["classification"]


class DensityComparisonTests(unittest.TestCase):
    def test_top_level_capture_and_handoff_failures_override_complete_owners(self):
        for key in ("capture_complete", "assessment_complete"):
            for value in (False, None, 1):
                inputs = fixture()
                inputs[1][key] = value
                report = compare(*inputs)
                self.assertEqual(report["counts"], {"uncovered": 1})
                self.assertFalse(report["successful"])
                self.assertFalse(report["fully_complete"])
            inputs = fixture()
            inputs[1].pop(key)
            self.assertEqual(classification(inputs), "uncovered")

    def test_malformed_input_envelopes_and_source_cases_do_not_crash(self):
        for index in range(4):
            for malformed in (None, [], "invalid", 7):
                inputs = list(fixture())
                inputs[index] = malformed
                result = compare(*inputs)
                self.assertFalse(result["successful"])
                self.assertFalse(result["fully_complete"])
        for malformed in (None, {}, "invalid", [], [None], [42], ["invalid"]):
            inputs = fixture()
            inputs[2]["source_cases"] = malformed
            report = compare(*inputs)
            self.assertEqual(report["counts"], {"uncovered": 1})
            self.assertFalse(report["successful"])
        inputs = fixture()
        inputs[2]["scope"] = []
        self.assertEqual(classification(inputs), "context-mismatch")
        inputs = fixture()
        inputs[2]["source_cases"][0]["expected"] = []
        self.assertEqual(classification(inputs), "uncovered")

    def test_exact_independent_measurement_agrees(self):
        report = compare(*fixture())
        self.assertEqual(report["counts"], {"evaluated-agreement": 1})
        self.assertTrue(report["fully_complete"])
        self.assertEqual(report["summary"]["measured"], 1)

    def test_wrong_count_denominator_ratio_or_outcome_is_regression(self):
        for location, key, value in (("measurement", "actual", 999), ("measurement", "outcome", "evaluated_no_finding"), ("source", "statements", 2), ("source", "code_lines", 4), ("event_inventory", "counts", {"mutation": 2})):
            with self.subTest(key=key):
                inputs = fixture()
                target = density(inputs)["measurement"]["source"] if location == "source" else density(inputs)[location]
                target[key] = value
                self.assertEqual(classification(inputs), "actual-regression")

    def test_numeric_bools_and_missing_evidence_are_uncovered(self):
        for key, value in (("actual", True), ("limit", False), ("exempt", 0)):
            inputs = fixture()
            density(inputs)["measurement"][key] = value
            self.assertEqual(classification(inputs), "uncovered")
        for section in ("event_inventory", "measurement"):
            inputs = fixture()
            density(inputs).pop(section)
            self.assertEqual(classification(inputs), "uncovered")

    def test_context_schema_and_effective_policy_must_match(self):
        inputs = fixture()
        inputs[3]["new"]["context"]["revision"] = "different"
        self.assertEqual(classification(inputs), "context-mismatch")
        for key, value in (("density_metric", "unknown"), ("statement_metric", None), ("schema", "rstyle.mutation-effects/3"), ("density_requested", False)):
            inputs = fixture()
            inputs[1]["mutation_effects"][key] = value
            self.assertEqual(classification(inputs), "uncovered")
        inputs = fixture()
        inputs[1]["mutation_effects"]["density_config"]["side_effects"]["weights"]["mutation"] = True
        self.assertEqual(classification(inputs), "uncovered")
        inputs = fixture()
        inputs[1]["mutation_effects"]["density_config"]["side_effects"]["paths"]["time"] = ["different"]
        self.assertEqual(classification(inputs), "uncovered")

    def test_duplicate_missing_and_ambiguous_owner_are_uncovered(self):
        for mode in ("duplicate", "missing", "legacy", "labels"):
            inputs = fixture()
            functions = inputs[1]["mutation_effects"]["functions"]
            if mode == "duplicate":
                functions.append(copy.deepcopy(functions[0]))
            elif mode == "missing":
                functions.clear()
            elif mode == "legacy":
                inputs[3]["legacy"]["functions"].clear()
            else:
                inputs[2]["source_cases"] *= 2
            self.assertEqual(classification(inputs), "uncovered")

    def test_unknown_zero_weight_cannot_become_numeric_clean(self):
        inputs = fixture()
        density(inputs)["event_inventory"].update(counts={"unknown": 1}, events=[{"kind": "unknown"}])
        self.assertEqual(classification(inputs), "uncovered")
        inputs = fixture()
        expected = inputs[2]["source_cases"][0]["expected"]
        expected.update(events={"unknown": 1}, actual=0, outcome="evaluated_no_finding")
        for context in (inputs[2]["scope"], inputs[3]["legacy"]["context"], inputs[3]["new"]["context"]):
            context["policy"]["weights"]["unknown"] = 0
        self.assertEqual(classification(inputs), "uncovered")

    def test_incomplete_budget_is_separate_expected_uncovered_denominator(self):
        inputs = fixture()
        inputs[1]["mutation_effects"]["execution"] = "budget_exhausted"
        function = inputs[1]["mutation_effects"]["functions"][0]
        function["evidence"]["execution"] = "budget_exhausted"
        raw = density(inputs)
        raw["complete"] = False
        raw["event_inventory"]["coverage"] = "incomplete"
        raw["measurement"].update(actual=None, outcome="not_evaluated")
        inputs[2]["source_cases"][0]["expected"].update(complete=False, actual=None, outcome="not_evaluated")
        report = compare(*inputs)
        self.assertEqual(report["counts"], {"expected-uncovered": 1})
        self.assertTrue(report["successful"])
        self.assertFalse(report["fully_complete"])
        self.assertEqual(report["summary"], dict(cases=1, measured=0, expected_uncovered=1, policy_excluded=0))
        inputs[1]["mutation_effects"]["execution"] = "failed"
        self.assertEqual(classification(inputs), "uncovered")

    def test_unexpected_incomplete_is_uncovered_not_clean(self):
        inputs = fixture()
        density(inputs)["complete"] = False
        self.assertEqual(classification(inputs), "uncovered")

    def test_source_suppression_preserves_raw_measurement(self):
        inputs = fixture()
        inputs[0]["diagnostics"] = []
        inputs[2]["source_cases"][0]["suppressed"] = True
        density(inputs).update(suppressed=True, visible_findings=0, source_suppression={"status": "allowed", "rules": ["side_effect_density"]})
        self.assertEqual(classification(inputs), "evaluated-agreement")
        density(inputs)["measurement"]["actual"] = 0
        self.assertEqual(classification(inputs), "actual-regression")

    def test_exclusion_requires_explicit_policy_and_raw_numeric_agreement(self):
        inputs = fixture()
        inputs[0]["diagnostics"] = []
        inputs[3]["legacy"]["functions"][0]["excluded"] = True
        inputs[2]["source_cases"][0]["excluded"] = True
        density(inputs)["source_exclusion"]["status"] = "excluded"
        density(inputs)["visible_findings"] = 0
        self.assertEqual(classification(inputs), "policy-excluded-agreement")
        density(inputs)["measurement"]["actual"] = 0
        self.assertEqual(classification(inputs), "actual-regression")

    def test_legacy_clean_and_diagnostic_have_different_evidence(self):
        inputs = fixture()
        inputs[0]["diagnostics"] = []
        self.assertEqual(classification(inputs), "unreviewed-disagreement")
        inputs[2]["source_cases"][0]["reviewed_difference"] = "Independent source count reviewed."
        self.assertEqual(classification(inputs), "intentional-difference")
        inputs[3]["legacy"]["complete"] = False
        self.assertEqual(classification(inputs), "uncovered")
        inputs = fixture()
        inputs[0]["diagnostics"][0].pop("actual")
        self.assertEqual(classification(inputs), "unreviewed-disagreement")
        inputs[0]["diagnostics"][0]["actual"] = 999
        self.assertEqual(classification(inputs), "unreviewed-disagreement")

    def test_missing_owner_assessment_and_malformed_rosters_are_uncovered(self):
        inputs = fixture()
        inputs[1]["mutation_effects"]["functions"][0].pop("evidence")
        self.assertEqual(classification(inputs), "uncovered")
        for malformed in (None, True, [], {"side_effects": None}):
            inputs = fixture()
            inputs[1]["mutation_effects"]["density_config"] = malformed
            self.assertEqual(classification(inputs), "uncovered")
        inputs = fixture()
        density(inputs)["event_inventory"]["events"] = [{"kind": []}]
        self.assertEqual(classification(inputs), "uncovered")

    def test_zero_known_weight_is_valid_but_unknown_remains_incomplete(self):
        inputs = fixture()
        for context in (inputs[2]["scope"], inputs[3]["legacy"]["context"], inputs[3]["new"]["context"]):
            context["policy"]["weights"]["mutation"] = 0
        inputs[1]["mutation_effects"]["density_config"]["side_effects"]["weights"]["mutation"] = 0
        inputs[2]["source_cases"][0]["expected"].update(actual=0, outcome="evaluated_no_finding")
        density(inputs)["measurement"].update(actual=0, outcome="evaluated_no_finding")
        density(inputs).update(visible_findings=0, findings=0)
        inputs[0]["diagnostics"] = []
        self.assertEqual(classification(inputs), "evaluated-agreement")
        inputs[3]["legacy"]["complete"] = False
        self.assertEqual(classification(inputs), "uncovered")

    def test_complete_minimum_exemption_is_validated(self):
        inputs = fixture()
        for context in (inputs[2]["scope"], inputs[3]["legacy"]["context"], inputs[3]["new"]["context"]):
            context["policy"]["min_lines"] = 4
        inputs[1]["mutation_effects"]["density_config"]["side_effects"]["min_lines"] = 4
        inputs[2]["source_cases"][0]["expected"].update(exempt=True, actual=None, outcome="evaluated_no_finding")
        density(inputs)["measurement"].update(exempt=True, actual=None, outcome="evaluated_no_finding")
        density(inputs).update(visible_findings=0, findings=0)
        inputs[0]["diagnostics"] = []
        self.assertEqual(classification(inputs), "evaluated-agreement")
        density(inputs)["measurement"]["exempt"] = False
        self.assertEqual(classification(inputs), "actual-regression")


if __name__ == "__main__":
    unittest.main()
