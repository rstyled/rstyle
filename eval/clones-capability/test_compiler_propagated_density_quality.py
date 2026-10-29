"""Source-labeled controls for reviewing completed propagated density results."""
from copy import deepcopy
import unittest

from compiler_quality import compare
from test_compiler_density_quality import fixture as direct_fixture, no_finding


def add_input(report, name, source, start):
    unit = report['compiler_advisory']['units'][0]
    key = f'function:u0:f{start}'
    span = {'start': start, 'end': start + len(source)}
    owner = deepcopy(report['functions'][0])
    owner.update(key=key, name=name, fn_span=span)
    report['functions'].append(owner)
    unit['observations'].append({'key': key, 'name': name, 'statements': 2})
    report['owned_source_inventory'][0]['source_functions'].append({
        'function_span': span, 'active_functions': [name],
        'source_file': '/workspace/src/lib.rs', 'source': source,
    })
    return {'package': 'sample', 'target_kind': 'lib', 'function': name,
            'source_file': 'src/lib.rs', 'source': source,
            'included': True, 'unsuppressed': True}


def fixture(depth=1):
    report, labels = direct_fixture()
    labels['density_policy']['transitive_depth'] = depth
    report['compiler_advisory']['effective_config']['side_effects']['transitive_depth'] = depth
    sample = labels['samples'][0]
    sample['source'] = 'fn f() {\n    helper();\n    ()\n}'
    report['owned_source_inventory'][0]['source_functions'][0]['source'] = sample['source']
    helper_source = 'fn helper() {\n    std::fs::read("a");\n    std::fs::read("b");\n}'
    if depth >= 2:
        helper_source = 'fn helper() {\n    leaf();\n    ()\n}'
    inputs = [add_input(report, 'helper', helper_source, 100)]
    if depth >= 2:
        leaf_source = ('fn leaf() {\n    last();\n    ()\n}' if depth == 3 else
                       'fn leaf() {\n    std::fs::read("a");\n    std::fs::read("b");\n}')
        inputs.append(add_input(report, 'leaf', leaf_source, 200))
    if depth == 3:
        inputs.append(add_input(report, 'last',
                                'fn last() {\n    std::fs::read("a");\n    std::fs::read("b");\n}', 300))
    details = [{'label': 'io: std::fs::read +1 more (via helper)', 'count': 1}]
    sample.update(propagation_reviewed=True, propagation_inputs=inputs)
    sample['density'].update(events={'io': 1}, actual=1500, details=deepcopy(details))
    diagnostic = report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]
    diagnostic.update(actual=1500, details=deepcopy(details))
    return report, labels


class CompilerPropagatedDensityQualityTests(unittest.TestCase):
    def test_explicit_propagation_input_target_is_not_substituted(self):
        report, labels = fixture()
        report['observed_units'][0]['target']['name'] = 'first'
        sample = labels['samples'][0]
        sample['target_name'] = 'first'
        sample['propagation_inputs'][0]['target_name'] = 'first'
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})
        sample['propagation_inputs'][0]['target_name'] = 'other'
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'unavailable': 1})
        self.assertIn('propagation input unavailable', result['comparisons'][0]['reason'])

    def test_overlapping_target_selectors_cannot_duplicate_a_propagation_owner(self):
        report, labels = fixture()
        report['observed_units'][0]['target']['name'] = 'first'
        inputs = labels['samples'][0]['propagation_inputs']
        duplicate = deepcopy(inputs[0])
        duplicate['target_name'] = 'first'
        inputs.append(duplicate)
        with self.assertRaisesRegex(ValueError, 'duplicate completed propagation input'):
            compare(report, labels)

    def test_depth_one_two_and_three_compare_completed_provenance_without_completing_scope(self):
        for depth in (1, 2, 3):
            with self.subTest(depth=depth):
                report, labels = fixture(depth)
                result = compare(report, labels)
                self.assertEqual(result['counts'], {'agreement': 1})
                self.assertTrue(result['sample_complete'])
                self.assertFalse(result['scope_complete'])
                self.assertFalse(result['planning_input_complete'])
                self.assertEqual(result['comparisons'][0]['observed'],
                                 {'outcome': 'finding', 'actual': 1500, 'limit': 500})

    def test_changed_via_leaf_or_collapsed_payload_is_disagreement(self):
        for label in ('io: std::fs::read +1 more (via other)',
                      'io: std::fs::read +1 more (via leaf)',
                      'io: std::fs::read +2 more (via helper)',
                      'io: std::fs::write +1 more (via helper)'):
            report, labels = fixture(2)
            report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]['details'][0]['label'] = label
            self.assertEqual(compare(report, labels)['counts'], {'disagreement': 1})

    def test_collapsed_payload_is_one_weighted_event(self):
        report, labels = fixture()
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})
        diagnostic = report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]
        diagnostic['details'][0]['count'] = 2
        self.assertEqual(compare(report, labels)['counts'], {'disagreement': 1})

    def test_missing_details_are_unavailable_and_extra_details_disagree(self):
        for details in (None, [], [{'label': 'network: socket (via helper)', 'count': 1}]):
            report, labels = fixture()
            diagnostic = report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]
            if details is None:
                diagnostic.pop('details')
            else:
                diagnostic['details'] = details
            expected = 'disagreement' if details else 'unavailable'
            self.assertEqual(compare(report, labels)['counts'], {expected: 1})

    def test_duplicate_observed_detail_cannot_preserve_agreement(self):
        report, labels = fixture()
        details = report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]['details']
        details.append(deepcopy(details[0]))
        self.assertEqual(compare(report, labels)['counts'], {'disagreement': 1})

    def test_expected_detail_roster_must_match_positive_weight_event_counts(self):
        for details in ([], [{'label': 'io: call (via helper)', 'count': 2}],
                        [{'label': 'network: call (via helper)', 'count': 1}],
                        [{'label': 'alien: call (via helper)', 'count': 1}]):
            report, labels = fixture()
            labels['samples'][0]['density']['details'] = details
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_boolean_zero_and_negative_detail_counts_are_rejected(self):
        for side in ('labels', 'report'):
            for count in (True, 0, -1, '1'):
                with self.subTest(side=side, count=count):
                    report, labels = fixture()
                    details = (labels['samples'][0]['density']['details'] if side == 'labels'
                               else report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]['details'])
                    details[0]['count'] = count
                    with self.assertRaises(ValueError):
                        compare(report, labels)

    def test_review_inputs_and_details_are_required_at_positive_depth(self):
        for field in ('propagation_reviewed', 'propagation_inputs', 'details'):
            report, labels = fixture()
            target = labels['samples'][0]['density'] if field == 'details' else labels['samples'][0]
            target.pop(field)
            with self.assertRaises(ValueError):
                compare(report, labels)
        report, labels = fixture()
        labels['samples'][0]['propagation_reviewed'] = False
        with self.assertRaises(ValueError):
            compare(report, labels)

    def test_reviewed_leaf_accepts_explicit_empty_inputs(self):
        report, labels = direct_fixture()
        labels['density_policy']['transitive_depth'] = 1
        report['compiler_advisory']['effective_config']['side_effects']['transitive_depth'] = 1
        sample = labels['samples'][0]
        sample.update(propagation_reviewed=True, propagation_inputs=[])
        sample['density']['details'] = [{'label': 'io: std::fs::read', 'count': 2}]
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})

    def test_missing_dependency_observation_stays_unavailable(self):
        report, labels = fixture()
        unit = report['compiler_advisory']['units'][0]
        dependency = unit['observations'].pop()
        unit['unavailable'].append({'key': dependency['key'], 'reason': 'callee body unavailable'})
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'unavailable': 1})
        self.assertFalse(result['sample_complete'])
        self.assertIsNone(result['comparisons'][0]['observed'])

    def test_missing_or_changed_dependency_source_is_unavailable(self):
        for changed in (False, True):
            report, labels = fixture()
            sources = report['owned_source_inventory'][0]['source_functions']
            if changed:
                sources[-1]['source'] += '\n'
            else:
                sources.pop()
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_ambiguous_dependency_owner_is_unavailable(self):
        report, labels = fixture()
        report['functions'].append(deepcopy(report['functions'][-1]))
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_dependency_inclusion_and_suppression_must_be_reviewed(self):
        for field in ('included', 'unsuppressed'):
            report, labels = fixture()
            labels['samples'][0]['propagation_inputs'][0][field] = False
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_unknown_expected_events_cannot_authorize_transitive_clean(self):
        report, labels = fixture()
        labels['samples'][0]['density']['events'] = {'unknown': 1}
        with self.assertRaises(ValueError):
            compare(report, labels)

    def test_depth_drift_unsupported_depth_and_boolean_depth_reject(self):
        for side, depth in (('report', 2), ('labels', 4), ('labels', True), ('report', False)):
            report, labels = fixture()
            policy = (labels['density_policy'] if side == 'labels'
                      else report['compiler_advisory']['effective_config']['side_effects'])
            policy['transitive_depth'] = depth
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_clean_result_does_not_invent_numeric_density_or_detail_evidence(self):
        report, labels = fixture()
        no_finding(report, labels, limit=1500)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 1})
        row = result['comparisons'][0]
        self.assertEqual(row['observed'], {'outcome': 'evaluated_no_finding'})
        self.assertNotIn('events', row)
        self.assertNotIn('details', row)

    def test_zero_weight_inventory_requires_explicit_empty_detail_labels(self):
        report, labels = fixture()
        no_finding(report, labels, limit=500)
        labels['samples'][0]['density'].update(events={'panic': 1}, actual=0, details=[])
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})

    def test_direct_contract_remains_without_propagation_fields_and_rejects_via(self):
        report, labels = direct_fixture()
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})
        report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0]['details'][0]['label'] += ' (via helper)'
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})


if __name__ == '__main__':
    unittest.main()
