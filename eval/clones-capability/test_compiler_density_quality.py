"""Independent controls for direct compiler-density quality samples."""
from copy import deepcopy
import unittest

from compiler_quality import compare
from test_compiler_quality import fixture as historical_fixture


def fixture():
    report, labels = historical_fixture()
    source = 'fn f() {\n    std::fs::read("a");\n    std::fs::read("b");\n}'
    policy = {
        'max_density_milli': 500, 'min_statements': 0, 'min_lines': 0,
        'transitive_depth': 0,
        'weights': {'mutation': 1, 'io': 3, 'network': 3, 'global': 2,
                    'unsafe': 4, 'time': 1, 'random': 1, 'panic': 0, 'unknown': 1},
        'paths': {'io': ['std::fs::'], 'network': [], 'time': [],
                  'random': [], 'panic': []},
        'exclude': ['target/**'],
    }
    labels['limits'] = {'side_effect_density': 500}
    labels['density_policy'] = deepcopy(policy)
    sample = labels['samples'][0]
    sample.pop('parameters')
    sample.pop('mutated_roots')
    sample.update(source=source, included=True, density={
        'events': {'io': 2}, 'complete': True, 'statements': 2,
        'code_lines': 4, 'actual': 3000, 'exempt': False, 'outcome': 'finding',
    })
    advisory = report['compiler_advisory']
    advisory['effective_config']['side_effects'] = {
        key: deepcopy(value) for key, value in policy.items() if key != 'exclude'
    }
    unit = advisory['units'][0]
    unit['observations'][0]['statements'] = 2
    unit['evaluation'] = {'summary': {'violations': 1}, 'diagnostics': [{
        'path': 'src/lib.rs', 'line': 1, 'rule_id': 'side_effect_density',
        'actual': 3000, 'limit': 500,
        'details': [{'label': 'io: std::fs::read', 'count': 2}],
    }]}
    advisory['planning']['groups'][0]['evaluation']['functions'][0]['rules'] = ['side_effect_density']
    report['owned_source_inventory'][0]['source_functions'][0]['source'] = source
    return report, labels


def no_finding(report, labels, limit=3000):
    labels['limits']['side_effect_density'] = limit
    labels['density_policy']['max_density_milli'] = limit
    labels['samples'][0]['density']['outcome'] = 'evaluated_no_finding'
    advisory = report['compiler_advisory']
    advisory['effective_config']['side_effects']['max_density_milli'] = limit
    advisory['planning']['groups'] = []
    advisory['units'][0]['evaluation'] = {'summary': {'violations': 0}, 'diagnostics': []}


class CompilerDensityQualityTests(unittest.TestCase):
    def test_direct_sample_checks_numeric_findings_without_completing_scope(self):
        report, labels = fixture()
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 1})
        self.assertTrue(result['sample_complete'])
        self.assertFalse(result['scope_complete'])
        self.assertFalse(result['planning_input_complete'])
        row = result['comparisons'][0]
        self.assertEqual(row['observed'], {'outcome': 'finding', 'actual': 3000, 'limit': 500})
        self.assertEqual(row['events'], {'expected': {'io': 2}, 'observed': {'io': 2}})

    def test_equal_limit_is_clean_without_synthesizing_numeric_density(self):
        report, labels = fixture()
        no_finding(report, labels)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 1})
        self.assertEqual(result['comparisons'][0]['observed'], {'outcome': 'evaluated_no_finding'})

    def test_each_size_exemption_is_clean_and_has_no_observed_number(self):
        for field, threshold in [('min_statements', 3), ('min_lines', 5)]:
            with self.subTest(field=field):
                report, labels = fixture()
                no_finding(report, labels, limit=500)
                labels['density_policy'][field] = threshold
                report['compiler_advisory']['effective_config']['side_effects'][field] = threshold
                labels['samples'][0]['density'].update(actual=None, exempt=True)
                result = compare(report, labels)
                self.assertEqual(result['counts'], {'agreement': 1})
                self.assertEqual(result['comparisons'][0]['observed'], {'outcome': 'evaluated_no_finding'})

    def test_equal_size_threshold_does_not_exempt_finding(self):
        report, labels = fixture()
        for field, threshold in [('min_statements', 2), ('min_lines', 4)]:
            labels['density_policy'][field] = threshold
            report['compiler_advisory']['effective_config']['side_effects'][field] = threshold
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})

    def test_missing_observation_and_source_preserve_unavailability(self):
        for missing in ('observation', 'source'):
            with self.subTest(missing=missing):
                report, labels = fixture()
                no_finding(report, labels)
                if missing == 'observation':
                    unit = report['compiler_advisory']['units'][0]
                    unit['observations'] = []
                    unit['unavailable'] = [{'key': 'function:u0:f0', 'reason': 'uncaptured body'}]
                else:
                    report['owned_source_inventory'] = []
                result = compare(report, labels)
                self.assertEqual(result['counts'], {'unavailable': 1})
                self.assertFalse(result['sample_complete'])
                self.assertIsNone(result['comparisons'][0]['observed'])

    def test_source_bytes_must_match_independent_sample(self):
        report, labels = fixture()
        report['owned_source_inventory'][0]['source_functions'][0]['source'] += '\n'
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'unavailable': 1})
        self.assertIn('source label differs', result['comparisons'][0]['reason'])

    def test_statement_count_is_checked_even_for_clean_functions(self):
        for clean in (False, True):
            report, labels = fixture()
            if clean:
                no_finding(report, labels)
            report['compiler_advisory']['units'][0]['observations'][0]['statements'] = 3
            self.assertEqual(compare(report, labels)['counts'], {'disagreement': 1})

    def test_event_count_and_kind_mismatches_are_disagreements(self):
        for replacement in ({'label': 'io: std::fs::read', 'count': 1},
                            {'label': 'network: socket', 'count': 2}):
            report, labels = fixture()
            report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]['details'] = [replacement]
            self.assertEqual(compare(report, labels)['counts'], {'disagreement': 1})

    def test_zero_weight_labels_do_not_require_visible_details(self):
        report, labels = fixture()
        labels['samples'][0]['density']['events']['panic'] = 1
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})

    def test_absent_or_unusable_details_are_unavailable(self):
        for details in (None, [], [{'label': 'alien: call', 'count': 2}],
                        [{'label': 'io: read (via helper)', 'count': 2}]):
            report, labels = fixture()
            diagnostic = report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]
            if details is None:
                diagnostic.pop('details')
            else:
                diagnostic['details'] = details
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_unknown_or_incomplete_independent_events_cannot_authorize_clean_results(self):
        for replacement in ({'events': {'unknown': 1}}, {'events': {'alien': 1}},
                            {'complete': False, 'actual': None, 'outcome': 'not_evaluated'}):
            report, labels = fixture()
            labels['samples'][0]['density'].update(replacement)
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_policy_drift_and_boolean_numeric_fields_are_rejected(self):
        for side, field, value in (
            ('report', 'transitive_depth', 1), ('labels', 'transitive_depth', True),
            ('report', 'min_lines', True), ('labels', 'min_statements', True),
            ('report', 'max_density_milli', 600), ('labels', 'max_density_milli', True),
            ('report', 'weights', {'io': 3}), ('labels', 'paths', {'io': ['other::']}),
        ):
            with self.subTest(side=side, field=field):
                report, labels = fixture()
                policy = (report['compiler_advisory']['effective_config']['side_effects']
                          if side == 'report' else labels['density_policy'])
                policy[field] = value
                with self.assertRaises(ValueError):
                    compare(report, labels)

    def test_unreviewed_source_policy_is_rejected(self):
        for field in ('included', 'unsuppressed'):
            report, labels = fixture()
            labels['samples'][0][field] = False
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_dropped_or_duplicate_plans_and_diagnostics_cannot_become_clean(self):
        for change in ('drop_plan', 'duplicate_plan', 'duplicate_diagnostic'):
            report, labels = fixture()
            advisory = report['compiler_advisory']
            groups = advisory['planning']['groups']
            if change == 'drop_plan':
                groups.clear()
            elif change == 'duplicate_plan':
                groups.append(deepcopy(groups[0]))
            else:
                evaluation = advisory['units'][0]['evaluation']
                evaluation['diagnostics'].append(deepcopy(evaluation['diagnostics'][0]))
                evaluation['summary']['violations'] = 2
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_historical_parameter_and_mutation_labels_remain_unchanged(self):
        report, labels = historical_fixture()
        original_report, original_labels = deepcopy(report), deepcopy(labels)
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 2})
        self.assertEqual(report, original_report)
        self.assertEqual(labels, original_labels)


if __name__ == '__main__':
    unittest.main()
