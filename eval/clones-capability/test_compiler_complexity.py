"""Offline complexity comparisons consume completed observations without rescoring."""
from copy import deepcopy
import unittest

from compiler_complexity import compare
from test_compiler_quality import fixture as quality_fixture


def fixture():
    report, labels = quality_fixture()
    labels.update(schema='rstyle.compiler-complexity-cases/1',
                  metrics='rstyle.compiler-style-metrics/6', complexity_policy_version=1,
                  limits={'max_cyclomatic': 2, 'max_cognitive': 2})
    source = 'fn f(a:bool,b:bool) { if a { if b {} } }'
    labels['samples'][0].update(test=False, source=source,
        expected={'available': True, 'policy_version': 1, 'cyclomatic': 3, 'cognitive': 3})
    report['owned_source_inventory'][0]['source_functions'][0]['source'] = source
    advisory = report['compiler_advisory']
    advisory['metrics'] = labels['metrics']
    advisory['effective_config']['complexity'] = deepcopy(labels['limits'])
    unit = advisory['units'][0]
    unit.update(expected_functions=1, unavailable_complexity=[], unresolved_callees=[])
    unit['evaluation']['summary'].update(functions=1, complexity_measured=1, complexity_unavailable=0)
    unit['evaluation']['diagnostics'] = [
        {'path': 'src/lib.rs', 'line': 1, 'rule_id': rule, 'actual': 3, 'limit': 2}
        for rule in ('cyclomatic_complexity', 'cognitive_complexity')]
    advisory['planning']['groups'][0]['evaluation']['functions'][0]['rules'] = [
        'cyclomatic_complexity', 'cognitive_complexity']
    return report, labels


class CompilerComplexityTests(unittest.TestCase):
    def test_explicit_source_target_name_is_required_by_shared_association(self):
        report, labels = fixture()
        report['observed_units'][0]['target']['name'] = 'first'
        labels['samples'][0]['target_name'] = 'first'
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 2})
        labels['samples'][0]['target_name'] = 'other'
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 2})

    def test_completed_independent_findings_agree_without_claiming_complete_scope(self):
        report, labels = fixture()
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2})
        self.assertFalse(result['scope_complete'])
        for row in result['comparisons']:
            self.assertEqual(row['observed'], {'outcome': 'finding', 'actual': 3, 'limit': 2})

    def test_missing_complexity_remains_unavailable_even_without_diagnostics(self):
        report, labels = fixture()
        unit = report['compiler_advisory']['units'][0]
        unit['unavailable_complexity'] = [{'key': 'function:u0:f0', 'reason': 'AST owner unavailable'}]
        unit['evaluation'] = {'diagnostics': [], 'summary': {'violations': 0}}
        report['compiler_advisory']['planning']['groups'] = []
        labels['samples'][0]['expected'].update(cyclomatic=1, cognitive=0)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'unavailable': 2})
        for row in result['comparisons']:
            self.assertIsNone(row['observed'])
            self.assertEqual(row['reason'], 'AST owner unavailable')

    def test_historical_metric_cannot_be_reinterpreted_as_measured_complexity(self):
        report, labels = fixture()
        for document in (report['compiler_advisory'], labels):
            document['metrics'] = 'rstyle.compiler-style-metrics/5'
        with self.assertRaisesRegex(ValueError, 'complexity metric unavailable'):
            compare(report, labels)

    def test_source_and_context_mismatches_remain_unavailable(self):
        for mutate in (
            lambda sample: sample.update(source='different authored function'),
            lambda sample: sample.update(source_file='src/other.rs'),
            lambda sample: sample.update(test=True),
        ):
            report, labels = fixture()
            mutate(labels['samples'][0])
            result = compare(report, labels)
            self.assertEqual(result['counts'], {'unavailable': 2})
            self.assertTrue(all(row['observed'] is None for row in result['comparisons']))

    def test_actual_threshold_and_outcome_mismatches_disagree(self):
        for field, value in [('actual', 4), ('limit', 1)]:
            report, labels = fixture()
            report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0][field] = value
            self.assertEqual(compare(report, labels)['counts'], {'agreement': 1, 'disagreement': 1})
        report, labels = fixture()
        labels['samples'][0]['expected']['cyclomatic'] = 2
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 1, 'disagreement': 1})
        self.assertEqual(result['comparisons'][0]['expected']['outcome'], 'evaluated_no_finding')

    def test_missing_completed_diagnostic_owner_rejects_comparison(self):
        report, labels = fixture()
        report['compiler_advisory']['planning']['groups'] = []
        with self.assertRaisesRegex(ValueError, 'incomplete completed rule-owner roster'):
            compare(report, labels)

    def test_clean_comparisons_do_not_invent_numeric_measurements(self):
        report, labels = fixture()
        report['compiler_advisory']['units'][0]['evaluation'] = {
            'diagnostics': [], 'summary': {'violations': 0}}
        report['compiler_advisory']['planning']['groups'] = []
        labels['samples'][0]['expected'].update(cyclomatic=1, cognitive=0)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2})
        for row in result['comparisons']:
            self.assertEqual(row['observed'], {'outcome': 'evaluated_no_finding'})


if __name__ == '__main__':
    unittest.main()
