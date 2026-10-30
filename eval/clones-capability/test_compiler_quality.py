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
            'evaluation': {'diagnostics': diagnostics, 'summary': {'violations': 2, 'functions': 1}}}
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
    unit['evaluation'] = {'diagnostics': [], 'summary': {'violations': 0, 'functions': 1}}
    labels['samples'][0].update(parameters=2, mutated_roots=[])


def spacing_fixture():
    report, labels = fixture()
    clean(report, labels)
    labels['limits'] = {'fn_spacing': 4}
    labels['samples'][0]['spacing'] = {'outcome': 'finding', 'actual': 5, 'limit': 4}
    labels['samples'][0]['source'] = 'fn f() { one(); two(); three(); four(); five(); }'
    report['owned_source_inventory'][0]['source_functions'][0]['source'] = labels['samples'][0]['source']
    advisory = report['compiler_advisory']
    advisory['effective_config']['fn_spacing'] = {'min_statements': 5}
    advisory['units'][0]['evaluation'] = {
        'summary': {'violations': 1, 'functions': 1},
        'diagnostics': [{'path': 'src/lib.rs', 'line': 1, 'rule_id': 'fn_spacing', 'actual': 5, 'limit': 4}],
        'fix_groups': [{'functions': [{'name': 'f', 'path': 'src/lib.rs', 'line': 1,
                                     'rules': ['fn_spacing']}]}]}
    return report, labels


class CompilerQualityTests(unittest.TestCase):
    def test_requested_plans_preserve_unrequested_core_diagnostics(self):
        report, labels = fixture()
        labels['limits'] = {'mutated_targets': 1}
        unit = report['compiler_advisory']['units'][0]
        unit['diagnostic_scope'] = {'request': {'rules': ['mutated_targets']},
                                    'diagnostic_indices': [1]}
        function = report['compiler_advisory']['planning']['groups'][0]['evaluation']['functions'][0]
        function['rules'] = ['mutated_targets']
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})
        self.assertEqual(unit['evaluation']['summary']['violations'], 2)
        function['rules'] = ['fn_params', 'mutated_targets']
        with self.assertRaisesRegex(ValueError, 'incomplete completed rule-owner roster'):
            compare(report, labels)

    def test_requested_diagnostic_projection_cannot_drop_or_repeat_findings(self):
        for indices in ([], [0], [1, 1], [2], [True], ['1']):
            report, labels = fixture()
            labels['limits'] = {'mutated_targets': 1}
            unit = report['compiler_advisory']['units'][0]
            unit['diagnostic_scope'] = {'request': {'rules': ['mutated_targets']},
                                        'diagnostic_indices': indices}
            with self.subTest(indices=indices), self.assertRaises(ValueError):
                compare(report, labels)

    def test_unrequested_labels_cannot_be_reported_clean(self):
        report, labels = fixture()
        unit = report['compiler_advisory']['units'][0]
        unit['diagnostic_scope'] = {'request': {'rules': ['mutated_targets']},
                                    'diagnostic_indices': [1]}
        with self.assertRaisesRegex(ValueError, 'reviewed rule outside requested scope'):
            compare(report, labels)

    def test_default_exclusions_do_not_make_retained_generated_source_clean(self):
        report, labels = fixture()
        clean(report, labels)
        labels['samples'][0]['source_file'] = 'target/generated.rs'
        source = report['owned_source_inventory'][0]['source_functions'][0]
        source['source_file'] = '/workspace/target/generated.rs'
        report['compiler_advisory']['units'][0]['evaluation']['summary']['functions'] = 0
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'unavailable': 2})
        self.assertTrue(all(row['observed'] is None for row in result['comparisons']))

    def test_missing_selected_population_is_unknown_without_completed_owners(self):
        report, labels = fixture()
        clean(report, labels)
        del report['compiler_advisory']['units'][0]['evaluation']['summary']['functions']
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'unavailable': 2})
        self.assertFalse(result['sample_complete'])

    def test_spacing_reuses_completed_location_without_a_planner_owner(self):
        report, labels = spacing_fixture()
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 1})
        self.assertEqual(result['comparisons'][0]['observed'],
                         {'outcome': 'finding', 'actual': 5, 'limit': 4})

    def test_spacing_clean_outcome_is_independent_of_the_statement_threshold(self):
        report, labels = spacing_fixture()
        unit = report['compiler_advisory']['units'][0]
        unit['evaluation'] = {'summary': {'violations': 0, 'functions': 1}, 'diagnostics': [], 'fix_groups': []}
        labels['samples'][0]['spacing']['outcome'] = 'evaluated_no_finding'
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 1})
        self.assertEqual(result['comparisons'][0]['observed'], {'outcome': 'evaluated_no_finding'})

    def test_spacing_wrong_actual_limit_and_outcome_disagree(self):
        for field, value in [('actual', 6), ('limit', 3)]:
            report, labels = spacing_fixture()
            report['compiler_advisory']['units'][0]['evaluation']['diagnostics'][0][field] = value
            self.assertEqual(compare(report, labels)['counts'], {'disagreement': 1})
        report, labels = spacing_fixture()
        labels['samples'][0]['spacing']['outcome'] = 'evaluated_no_finding'
        self.assertEqual(compare(report, labels)['counts'], {'disagreement': 1})

    def test_spacing_labels_and_policy_require_exact_valid_values(self):
        for field, value in [('actual', True), ('limit', 3), ('outcome', 'unknown')]:
            report, labels = spacing_fixture()
            labels['samples'][0]['spacing'][field] = value
            with self.assertRaises(ValueError):
                compare(report, labels)
        report, labels = spacing_fixture()
        report['compiler_advisory']['effective_config']['fn_spacing']['min_statements'] = 0
        with self.assertRaises(ValueError):
            compare(report, labels)

    def test_spacing_incomplete_or_ambiguous_completed_locations_cannot_become_clean(self):
        for mutate in (
            lambda evaluation: evaluation.update(fix_groups=[]),
            lambda evaluation: evaluation['fix_groups'].append(deepcopy(evaluation['fix_groups'][0])),
            lambda evaluation: evaluation['fix_groups'][0]['functions'][0].update(line=2),
            lambda evaluation: evaluation['diagnostics'].append(deepcopy(evaluation['diagnostics'][0])),
        ):
            report, labels = spacing_fixture()
            evaluation = report['compiler_advisory']['units'][0]['evaluation']
            mutate(evaluation)
            evaluation['summary']['violations'] = len(evaluation['diagnostics'])
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_spacing_missing_or_mismatched_source_stays_unavailable(self):
        for mutate in (
            lambda report: report.update(owned_source_inventory=[]),
            lambda report: report['owned_source_inventory'][0]['source_functions'][0].update(source='other'),
        ):
            report, labels = spacing_fixture()
            mutate(report)
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_spacing_locations_cannot_share_a_diagnostic_and_leave_another_unowned(self):
        report, labels = spacing_fixture()
        unit = report['compiler_advisory']['units'][0]
        unit['observations'].append({'key': 'function:u0:f1', 'name': 'g'})
        functions = unit['evaluation']['fix_groups'][0]['functions']
        functions.append({**deepcopy(functions[0]), 'name': 'g'})
        diagnostics = unit['evaluation']['diagnostics']
        diagnostics.append({**deepcopy(diagnostics[0]), 'line': 2})
        unit['evaluation']['summary']['violations'] = 2
        with self.assertRaises(ValueError):
            compare(report, labels)

    def test_explicit_test_context_selects_its_own_owner_and_inventory(self):
        report, labels = spacing_fixture()
        unit = deepcopy(report['compiler_advisory']['units'][0])
        unit.update(unit=1, observations=[{'key': 'function:u1:f0', 'name': 'f'}])
        unit['evaluation'] = {'summary': {'violations': 0, 'functions': 1}, 'diagnostics': [], 'fix_groups': []}
        report['compiler_advisory']['units'].append(unit)
        report['unit_contexts'].append({'id': 1, 'observed_unit': 1})
        report['observed_units'].append({**deepcopy(report['observed_units'][0]), 'test': True})
        owner = deepcopy(report['functions'][0])
        owner.update(key='function:u1:f0', unit_evidence={'report_unit': 1})
        report['functions'].append(owner)
        inventory = deepcopy(report['owned_source_inventory'][0])
        inventory['test'] = True
        report['owned_source_inventory'].append(inventory)
        sample = deepcopy(labels['samples'][0])
        sample.update(test=True, spacing={'outcome': 'evaluated_no_finding', 'actual': 5, 'limit': 4})
        labels['samples'].append(sample)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2})
        self.assertEqual([row['test'] for row in result['comparisons']], [False, True])
        report['owned_source_inventory'].pop()
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1, 'unavailable': 1})

    def test_explicit_target_name_is_checked_in_shared_source_association(self):
        report, labels = spacing_fixture()
        report['observed_units'][0]['target']['name'] = 'first'
        labels['samples'][0]['target_name'] = 'first'
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1})
        labels['samples'][0]['target_name'] = 'other'
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})
        for invalid in (None, '', False, 7):
            labels['samples'][0]['target_name'] = invalid
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_overlapping_named_and_unnamed_selectors_cannot_double_count_an_owner(self):
        report, labels = spacing_fixture()
        report['observed_units'][0]['target']['name'] = 'first'
        duplicate = deepcopy(labels['samples'][0])
        duplicate['target_name'] = 'first'
        labels['samples'].append(duplicate)
        with self.assertRaisesRegex(ValueError, 'duplicate completed source owner'):
            compare(report, labels)

    def test_same_named_source_in_distinct_targets_keeps_exact_context(self):
        report, labels = spacing_fixture()
        report['observed_units'][0]['target']['kind'] = ['bin']
        labels['samples'][0]['target_kind'] = 'bin'
        report['observed_units'][0]['target']['name'] = 'first'
        second = deepcopy(report['compiler_advisory']['units'][0])
        second.update(unit=1, observations=[{'key': 'function:u1:f0', 'name': 'f'}])
        second['evaluation'] = {'summary': {'violations': 0, 'functions': 1}, 'diagnostics': [], 'fix_groups': []}
        report['compiler_advisory']['units'].append(second)
        report['unit_contexts'].append({'id': 1, 'observed_unit': 1})
        observed = deepcopy(report['observed_units'][0])
        observed['target']['name'] = 'second'
        report['observed_units'].append(observed)
        owner = deepcopy(report['functions'][0])
        owner.update(key='function:u1:f0', unit_evidence={'report_unit': 1})
        report['functions'].append(owner)
        inventory = deepcopy(report['owned_source_inventory'][0])
        inventory['compilation_unit'] = deepcopy(observed['target'])
        report['owned_source_inventory'].append(inventory)
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})
        labels['samples'][0]['target_name'] = 'first'
        sample = deepcopy(labels['samples'][0])
        sample.update(target_name='second', spacing={
            'outcome': 'evaluated_no_finding', 'actual': 5, 'limit': 4})
        labels['samples'].append(sample)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2})
        self.assertEqual([row['key'] for row in result['comparisons']],
                         ['function:u0:f0', 'function:u1:f0'])
        self.assertEqual([row['target_name'] for row in result['comparisons']], ['first', 'second'])
        report['owned_source_inventory'].pop()
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 1, 'unavailable': 1})

    def test_missing_ambiguous_and_invalid_context_selectors_are_not_guessed(self):
        report, labels = spacing_fixture()
        labels['samples'][0]['test'] = True
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})
        labels['samples'][0]['test'] = 'true'
        with self.assertRaises(ValueError):
            compare(report, labels)
        report, labels = spacing_fixture()
        report['compiler_advisory']['units'].append(deepcopy(report['compiler_advisory']['units'][0]))
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

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

    def test_duplicate_json_keys_fail_before_cli_publication(self):
        report, labels = fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            encoded = json.dumps(report).replace('"test": false',
                                                  '"test": true, "test": false', 1)
            self.assertNotEqual(encoded, json.dumps(report))
            (root / 'report').write_text(encoded)
            (root / 'labels').write_text(json.dumps(labels))
            command = [sys.executable, str(Path(__file__).with_name('compiler_quality.py')),
                       '--report', str(root / 'report'), '--labels', str(root / 'labels'),
                       '--output', str(root / 'output')]
            completed = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(completed.returncode, 2)
            self.assertIn('duplicate JSON object key: test', completed.stderr)
            self.assertFalse((root / 'output').exists())


if __name__ == '__main__':
    unittest.main()
