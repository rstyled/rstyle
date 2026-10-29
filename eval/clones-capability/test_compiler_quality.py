"""Independent contracts for offline review of completed compiler observations."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from compiler_quality import compare


def fixture():
    key = 'function:u0:f0'
    span = {'start': 0, 'end': 8}
    target = {'package_context': {'name': 'sample'}, 'kind': ['lib']}
    function = {'name': 'f', 'path': 'src/lib.rs', 'line': 1,
                'rules': ['fn_params', 'mutated_targets']}
    diagnostics = [{'path': 'src/lib.rs', 'line': 1, 'rule_id': rule,
                    'actual': actual, 'limit': limit}
                   for rule, actual, limit in [('fn_params', 3, 2), ('mutated_targets', 2, 1)]]
    diagnostics[1]['details'] = [{'label': 'a'}, {'label': 'b'}]
    unit = {'unit': 0, 'source_root': '/workspace',
            'observations': [{'key': key, 'name': 'f'}], 'unavailable': [],
            'evaluation': {'diagnostics': diagnostics, 'summary': {'violations': 2}}}
    report = {'run_directory': '/saved/run-sample', 'toolchain': 'nightly',
              'unit_contexts': [{'id': 0, 'observed_unit': 0}],
              'observed_units': [{'target': target, 'test': False}],
              'functions': [{'key': key, 'name': 'f', 'fn_span': span, 'outcome': 'excluded',
                             'unit_evidence': {'report_unit': 0}}],
              'owned_source_inventory': [{'compilation_unit': target, 'test': False,
                  'source_functions': [{'function_span': span, 'active_functions': ['f'],
                                        'source_file': '/workspace/src/lib.rs'}]}],
              'compiler_advisory': {'schema': 'rstyle.compiler-advisory/1', 'metrics': 'metric/1',
                  'complete': False, 'units': [unit],
                  'effective_config': {'exclude': ['target/**'],
                      'fn_params': {'max_params': 2, 'count_self': False},
                      'mutation_budget': {'max_mutated_targets': 1, 'granularity': 'root'}},
                  'planning': {'schema': 'rstyle.compiler-planning/1', 'input_complete': False,
                      'groups': [{'members': [key], 'evaluation': {'functions': [function]}}]}}}
    labels = {'schema': 'rstyle.compiler-quality-cases/1', 'run': 'run-sample',
              'toolchain': 'nightly', 'metrics': 'metric/1',
              'limits': {'fn_params': 2, 'mutated_targets': 1},
              'samples': [{'package': 'sample', 'target_kind': 'lib', 'function': 'f',
                           'source_file': 'src/lib.rs', 'parameters': 3,
                           'mutated_roots': ['a', 'b'], 'unsuppressed': True}],
              'limitations': 'Sample only; no precision claim.', 'preconditions': 'Unsuppressed original owner.'}
    return report, labels


def clean(report, labels):
    report['compiler_advisory']['planning']['groups'] = []
    unit = report['compiler_advisory']['units'][0]
    unit['evaluation'] = {'diagnostics': [], 'summary': {'violations': 0}}
    labels['samples'][0].update(parameters=2, mutated_roots=[])


class CompilerQualityTests(unittest.TestCase):
    def test_sample_agreement_does_not_make_incomplete_scope_clean(self):
        report, labels = fixture()
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2})
        self.assertTrue(result['sample_complete'])
        self.assertFalse(result['scope_complete'])
        self.assertFalse(result['planning_input_complete'])

    def test_known_clean_does_not_synthesize_numeric_zero(self):
        report, labels = fixture()
        clean(report, labels)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2})
        for row in result['comparisons']:
            self.assertEqual(row['observed'], {'outcome': 'evaluated_no_finding'})

    def test_missing_observation_preserves_reason_and_unavailability(self):
        report, labels = fixture()
        clean(report, labels)
        unit = report['compiler_advisory']['units'][0]
        unit['observations'] = []
        unit['unavailable'] = [{'key': 'function:u0:f0', 'reason': 'generated source'}]
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'unavailable': 2})
        self.assertFalse(result['sample_complete'])
        self.assertTrue(all(row['reason'] == 'generated source' for row in result['comparisons']))

    def test_wrong_count_and_wrong_outcome_are_disagreements(self):
        for parameters in (2, 4):
            report, labels = fixture()
            labels['samples'][0]['parameters'] = parameters
            self.assertEqual(compare(report, labels)['counts'], {'agreement': 1, 'disagreement': 1})

    def test_target_provenance_is_checked_and_missing_details_stay_unavailable(self):
        report, labels = fixture()
        diagnostic = report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][1]
        diagnostic['details'][1]['label'] = 'other'
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1, 'disagreement': 1})
        diagnostic.pop('details')
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1, 'unavailable': 1})

    def test_generated_root_can_turn_source_clean_into_reported_finding(self):
        report, labels = fixture()
        labels['limits']['mutated_targets'] = 2
        advisory = report['compiler_advisory']
        advisory['effective_config']['mutation_budget']['max_mutated_targets'] = 2
        diagnostic = advisory['units'][0]['evaluation']['diagnostics'][1]
        diagnostic.update(actual=3, limit=2)
        diagnostic['details'].append({'label': 'iter'})
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 1, 'disagreement': 1})
        row = result['comparisons'][1]
        self.assertEqual(row['expected']['outcome'], 'evaluated_no_finding')
        self.assertEqual(row['observed']['outcome'], 'finding')
        self.assertEqual(row['observed']['roots'], ['a', 'b', 'iter'])

    def test_empty_sample_and_invalid_numeric_limits_are_rejected(self):
        report, labels = fixture()
        labels['samples'] = []
        with self.assertRaises(ValueError):
            compare(report, labels)
        report, labels = fixture()
        labels['limits']['mutated_targets'] = True
        with self.assertRaises(ValueError):
            compare(report, labels)

    def test_wrong_context_or_policy_rejects_comparison(self):
        for mutate in (
            lambda report: report.update(toolchain='other'),
            lambda report: report.update(run_directory='/saved/other'),
            lambda report: report['compiler_advisory'].update(metrics='other'),
            lambda report: report['compiler_advisory'].update(complete='false'),
            lambda report: report['compiler_advisory']['effective_config'].update(exclude=['src/**']),
        ):
            report, labels = fixture()
            mutate(report)
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_incomplete_or_duplicate_planned_roster_cannot_become_clean(self):
        for mutate in (
            lambda planning: planning.update(groups=[]),
            lambda planning: planning['groups'][0].update(members=[]),
            lambda planning: planning['groups'].append(deepcopy(planning['groups'][0])),
        ):
            report, labels = fixture()
            mutate(report['compiler_advisory']['planning'])
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_duplicate_diagnostic_does_not_gain_a_unique_owner(self):
        report, labels = fixture()
        unit = report['compiler_advisory']['units'][0]
        unit['evaluation']['diagnostics'].append(deepcopy(unit['evaluation']['diagnostics'][0]))
        unit['evaluation']['summary']['violations'] = 3
        with self.assertRaises(ValueError):
            compare(report, labels)

    def test_missing_source_association_is_unavailable(self):
        report, labels = fixture()
        report['owned_source_inventory'] = []
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 2})

    def test_duplicate_source_names_or_units_are_not_chosen_by_order(self):
        report, labels = fixture()
        report['functions'].append(deepcopy(report['functions'][0]))
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 2})
        report, labels = fixture()
        report['compiler_advisory']['units'].append(deepcopy(report['compiler_advisory']['units'][0]))
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 2})

    def test_unreviewed_suppression_or_duplicate_labels_reject(self):
        report, labels = fixture()
        labels['samples'][0]['unsuppressed'] = False
        with self.assertRaises(ValueError):
            compare(report, labels)
        report, labels = fixture()
        labels['samples'].append(deepcopy(labels['samples'][0]))
        with self.assertRaises(ValueError):
            compare(report, labels)

    def test_cli_writes_once_and_distinguishes_disagreement(self):
        report, labels = fixture()
        labels['samples'][0]['parameters'] = 2
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, value in [('report', report), ('labels', labels)]:
                (root / name).write_text(json.dumps(value))
            command = [sys.executable, str(Path(__file__).with_name('compiler_quality.py')),
                       '--report', str(root / 'report'), '--labels', str(root / 'labels'),
                       '--output', str(root / 'output')]
            first = subprocess.run(command, capture_output=True)
            self.assertEqual(first.returncode, 1, first.stderr)
            self.assertEqual(json.loads((root / 'output').read_text())['counts']['disagreement'], 1)
            saved = (root / 'output').read_bytes()
            second = subprocess.run(command, capture_output=True)
            self.assertEqual(second.returncode, 2)
            self.assertEqual((root / 'output').read_bytes(), saved)


if __name__ == '__main__':
    unittest.main()
