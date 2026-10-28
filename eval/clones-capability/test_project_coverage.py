"""Independent synthetic contracts for the project-study reducer."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest

from project_coverage import summarize, load

ROOT = Path('/workspace')


def fixture():
    target = {'name': 'sample', 'kind': ['lib'], 'src_path': '/workspace/src/lib.rs'}
    observed = {'name': 'sample', 'kind': ['lib'], 'source': target['src_path'],
                'package': 'sample-id', 'package_context': {'name': 'sample'}}
    policy = {'max_lines': 10, 'max_params': 1, 'count_self': False,
              'min_statements': 5, 'exclude': []}
    labels = {'schema': 'rstyle.clones-capability.project-study-cases/1',
              'package': 'sample', 'target': 'lib', 'features': [], 'source_policy': policy,
              'samples': [{'function': 'example::f', 'source_file': 'src/lib.rs',
                           'expected': {'fn_length': {'actual': 3, 'limit': 10,
                                                     'outcome': 'evaluated-no-finding'}}}]}
    key = 'function:u0:f0'
    span = {'start': 0, 'end': 20}
    assessments = [{'rule_id': rule, 'outcome': 'evaluated_no_finding', 'actual': actual,
                    'limit': limit, 'availability': 'known', 'coverage': 'complete', 'reasons': []}
                   for rule, actual, limit in [('fn_length', 3, 10), ('fn_params', 0, 1), ('fn_spacing', 1, 4)]]
    exclusion = {'status': 'included', 'path': 'src/lib.rs'}
    suppression = {'status': 'none', 'comment': ''}
    report = {'scope': {'packages': ['sample'], 'all_targets': False,
                        'build_options': {'source_rules': True, 'effect_density': True,
                                          'guard_clauses': False, 'mutation_effects': False}},
              'intended_primary_units': [{'mode': 'check', 'pkg_id': 'sample-id', 'target': target, 'features': []}],
              'observed_units': [{'target': observed, 'test': False}],
              'unit_contexts': [{'id': 0, 'observed_unit': 0}],
              'functions': [{'key': key, 'name': 'example::f', 'fn_span': span,
                             'source_digest': 'checksum', 'compilation_unit': 'sample-id:sample:test=false',
                             'unit_evidence': {'report_unit': 0, 'function': 0}}],
              'capture_complete': True, 'assessment_complete': True,
              'pair_enumeration_complete': True, 'pair_evaluation_complete': True,
              'analysis': {'schema': 'rstyle.consumer-analysis/1', 'execution': 'completed',
                           'scope': {'coverage': 'complete', 'reasons': [], 'assessed_functions': 1,
                                     'expected_functions': 1, 'evaluated_rules': 3,
                                     'unavailable_rules': 0, 'excluded_rules': 0},
                           'findings': 0, 'visible_findings': 0, 'suppressed_findings': 0,
                           'effective_config': policy,
                           'functions': [{'key': key, 'name': 'example::f', 'fn_span': span,
                                          'source_digest': 'checksum', 'source_file': '/workspace/src/lib.rs',
                                          'execution': 'completed', 'assessments': assessments,
                                          'source_exclusion': exclusion, 'source_suppression': suppression}]},
              'mutation_effects': {'schema': 'rstyle.mutation-effects/4', 'density_requested': True,
                                   'execution': 'completed', 'coverage': 'complete', 'reasons': [],
                                   'scalar_mutations_requested': False, 'assessed_functions': 1,
                                   'evaluated_functions': 1, 'unavailable_functions': 0,
                                   'findings': 0, 'visible_findings': 0, 'review_hidden_findings': 0,
                                   'expected_functions': 1, 'density_config': {'exclude': []},
                                   'functions': [{'key': key, 'name': 'example::f', 'fn_span': span,
                                                  'source_file': '/workspace/src/lib.rs',
                                                  'evidence': {'execution': 'completed', 'coverage': 'complete',
                                                               'availability': {'status': 'known', 'value': None},
                                                               'signature': {'status': 'known', 'value': None}, 'reasons': []},
                                                  'statement_inventory': {'execution': 'completed',
                                                                          'statements': {'status': 'known', 'value': 1}},
                                                  'density': {'complete': True, 'findings': 0, 'visible_findings': 0,
                                                              'invalid_suppression': None,
                                                              'event_inventory': {'coverage': 'complete', 'counts': {}},
                                                              'measurement': {'outcome': 'evaluated_no_finding',
                                                                              'actual': 0, 'limit': 500,
                                                                              'source': {'statements': 1, 'code_lines': 3}},
                                                              'source_exclusion': exclusion,
                                                              'source_suppression': suppression}}]}}
    return report, labels


class ProjectCoverageTests(unittest.TestCase):
    def test_full_roster_and_sample_are_separate(self):
        report, labels = fixture()
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['roster']['denominator'], 1)
        self.assertEqual(result['quality']['counts'], {'agreement': 1})
        self.assertEqual(result['source']['rules']['fn_length']['known_evaluated'], 1)
        self.assertEqual(result['density']['counts']['known_evaluated'], 1)

    def test_failed_handoff_has_unknown_denominator(self):
        report, labels = fixture()
        report['assessment_complete'] = False
        report['functions'] = []
        report['handoff_failures'] = ['native evidence unavailable']
        for consumer in ('analysis', 'mutation_effects'):
            value = report[consumer]
            value['functions'] = []
            value['execution'] = 'failed'
            scope = value.get('scope', value)
            scope.update(expected_functions=None, assessed_functions=0, coverage='incomplete')
            if consumer == 'analysis':
                scope['evaluated_rules'] = 0
            else:
                value['evaluated_functions'] = 0
        result = summarize(report, labels, ROOT)
        self.assertIsNone(result['roster']['denominator'])
        self.assertEqual(result['quality']['counts'], {'unobserved': 1})
        self.assertEqual(result['failures']['handoff'], ['native evidence unavailable'])

    def test_missing_rows_are_missing_not_clean(self):
        report, labels = fixture()
        report['analysis']['functions'] = []
        report['analysis']['scope'].update(assessed_functions=0, evaluated_rules=0)
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['source']['rules']['fn_length'], {'missing': 1})
        self.assertEqual(result['source']['coverage'], 'incomplete')
        self.assertEqual(result['roster']['denominator'], 1)
        self.assertEqual(result['quality']['counts'], {'unavailable': 1})

    def test_missing_rule_is_unavailable(self):
        report, labels = fixture()
        report['analysis']['functions'][0]['assessments'].pop(0)
        report['analysis']['scope']['evaluated_rules'] = 2
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['quality']['counts'], {'unavailable': 1})

    def test_exact_names_and_paths(self):
        report, labels = fixture()
        labels['samples'][0]['function'] = 'f'
        self.assertEqual(summarize(report, labels, ROOT)['quality']['counts'], {'unobserved': 1})
        labels['samples'][0]['source_file'] = '../src/lib.rs'
        with self.assertRaisesRegex(ValueError, 'traversal'):
            summarize(report, labels, ROOT)

    def test_duplicate_and_extra_rows_fail(self):
        for section in ('functions', 'analysis', 'mutation_effects'):
            with self.subTest(section=section):
                report, labels = fixture()
                rows = report[section] if section == 'functions' else report[section]['functions']
                rows.append(deepcopy(rows[0]))
                with self.assertRaisesRegex(ValueError, 'duplicate'):
                    summarize(report, labels, ROOT)
        report, labels = fixture()
        report['analysis']['functions'][0]['key'] = 'function:u0:f99'
        with self.assertRaisesRegex(ValueError, 'extra'):
            summarize(report, labels, ROOT)

    def test_context_mismatches_fail(self):
        for mutate in (lambda r: r['observed_units'][0].update(test=True),
                       lambda r: r['intended_primary_units'][0].update(features=['extra']),
                       lambda r: r['unit_contexts'][0].update(observed_unit=2),
                       lambda r: r['functions'][0].update(key='function:u1:f0')):
            report, labels = fixture()
            mutate(report)
            with self.assertRaises(ValueError):
                summarize(report, labels, ROOT)

    def test_booleans_are_not_measurements(self):
        report, labels = fixture()
        report['analysis']['functions'][0]['assessments'][0]['actual'] = True
        with self.assertRaisesRegex(ValueError, 'integer'):
            summarize(report, labels, ROOT)

    def test_unknown_density_stays_outside_measured_denominator(self):
        report, labels = fixture()
        value = report['mutation_effects']['functions'][0]['density']
        value.update(complete=False)
        value['event_inventory']['coverage'] = 'incomplete'
        value['measurement'].update(outcome='not_evaluated', actual=None)
        report['mutation_effects'].update(evaluated_functions=0, unavailable_functions=1)
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['density']['counts']['known_evaluated'], 0)
        self.assertEqual(result['density']['counts']['not_evaluated'], 1)

    def test_contradictory_density_fails(self):
        report, labels = fixture()
        report['mutation_effects']['functions'][0]['density']['event_inventory']['coverage'] = 'incomplete'
        with self.assertRaisesRegex(ValueError, 'incomplete event'):
            summarize(report, labels, ROOT)

    def test_exclusion_and_suppression_do_not_erase_measurements(self):
        report, labels = fixture()
        row = report['analysis']['functions'][0]
        row['source_suppression'] = {'status': 'allowed', 'rules': ['fn_length']}
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['source']['rules']['fn_length']['suppressed'], 1)
        self.assertEqual(result['quality']['counts'], {'agreement': 1})

    def test_disagreement_preserves_independent_labels(self):
        report, labels = fixture()
        labels['samples'][0]['expected']['fn_length']['actual'] = 4
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['quality']['counts'], {'disagreement': 1})
        self.assertEqual(result['quality']['comparisons'][0]['expected']['actual'], 4)

    def test_source_policy_and_identity_mismatch_fail(self):
        report, labels = fixture()
        report['analysis']['effective_config'] = {**labels['source_policy'], 'max_lines': 99}
        with self.assertRaisesRegex(ValueError, 'policy differs'):
            summarize(report, labels, ROOT)
        report, labels = fixture()
        report['analysis']['functions'][0]['source_digest'] = 'another source'
        with self.assertRaisesRegex(ValueError, 'source mismatch'):
            summarize(report, labels, ROOT)

    def test_statement_denominator_must_be_known_and_equal(self):
        report, labels = fixture()
        row = report['mutation_effects']['functions'][0]
        row['statement_inventory']['statements']['value'] = 2
        with self.assertRaisesRegex(ValueError, 'denominator mismatch'):
            summarize(report, labels, ROOT)
        row['statement_inventory']['statements'] = {'status': 'unknown'}
        with self.assertRaisesRegex(ValueError, 'unavailable statement'):
            summarize(report, labels, ROOT)

    def test_excluded_and_unavailable_source_policy_are_annotations(self):
        report, labels = fixture()
        row = report['analysis']['functions'][0]
        row['source_exclusion'] = {'status': 'excluded'}
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['source']['rules']['fn_length']['excluded'], 1)
        self.assertEqual(result['source']['rules']['fn_length']['known_evaluated'], 0)
        row['source_exclusion'] = {'status': 'unavailable'}
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['source']['rules']['fn_length']['source_policy_unavailable'], 1)

    def test_ambiguous_sample_owners_fail(self):
        report, labels = fixture()
        owner = deepcopy(report['functions'][0])
        owner['key'] = 'function:u0:f1'
        owner['unit_evidence']['function'] = 1
        report['functions'].append(owner)
        row = deepcopy(report['analysis']['functions'][0])
        row['key'] = owner['key']
        report['analysis']['functions'].append(row)
        report['analysis']['scope'].update(expected_functions=2, assessed_functions=2, evaluated_rules=6)
        report['mutation_effects']['expected_functions'] = 2
        with self.assertRaisesRegex(ValueError, 'ambiguous sample'):
            summarize(report, labels, ROOT)

    def test_cli_publication_is_create_new(self):
        report, labels = fixture()
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            report_path, label_path, output = [directory / name for name in ('report.json', 'labels.json', 'out.json')]
            report_path.write_text(json.dumps(report))
            label_path.write_text(json.dumps(labels))
            command = [sys.executable, str(Path(__file__).with_name('project_coverage.py')),
                       '--report', str(report_path), '--labels', str(label_path),
                       '--workspace-root', str(ROOT), '--output', str(output)]
            self.assertEqual(subprocess.run(command, capture_output=True).returncode, 0)
            original = output.read_bytes()
            self.assertEqual(subprocess.run(command, capture_output=True).returncode, 2)
            self.assertEqual(output.read_bytes(), original)

    def test_source_budget_rows_remain_in_roster_without_assessed_count(self):
        report, labels = fixture()
        source = report['analysis']
        row = source['functions'][0]
        row['execution'] = 'budget_exhausted'
        for item in row['assessments']:
            item.update(outcome='not_evaluated', actual=None, availability='unknown', coverage='incomplete')
        source['scope'].update(assessed_functions=0, evaluated_rules=0, unavailable_rules=3, coverage='incomplete')
        source['execution'] = 'budget_exhausted'
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['roster']['denominator'], 1)
        self.assertEqual(result['source']['rules']['fn_length']['budget_exhausted'], 1)
        self.assertEqual(result['quality']['counts'], {'unavailable': 1})
        comparison = result['quality']['comparisons'][0]
        self.assertEqual(comparison['function_execution'], 'budget_exhausted')
        self.assertEqual(comparison['assessment_evidence']['availability'], 'unknown')

    def test_checked_exclusion_does_not_require_suppression_observation(self):
        report, labels = fixture()
        row = report['analysis']['functions'][0]
        row['source_exclusion'] = {'status': 'excluded'}
        row.pop('source_suppression')
        for item in row['assessments']:
            item.update(outcome='not_evaluated', actual=None, availability='not_applicable')
        report['analysis']['scope'].update(evaluated_rules=0, excluded_rules=3)
        result = summarize(report, labels, ROOT)
        count = result['source']['rules']['fn_length']
        self.assertEqual(count['excluded'], 1)
        self.assertEqual(count['source_policy_unavailable'], 0)

    def test_invalid_suppression_is_not_unavailable(self):
        report, labels = fixture()
        row = report['analysis']['functions'][0]
        row['source_suppression'] = {'status': 'invalid', 'reason': 'bad rule'}
        row['assessments'].append({'rule_id': 'invalid_suppression', 'outcome': 'finding',
                                   'coverage': 'complete', 'availability': 'known', 'actual': 1, 'limit': 0})
        report['analysis'].update(findings=1, visible_findings=1)
        report['analysis']['scope']['evaluated_rules'] = 4
        result = summarize(report, labels, ROOT)
        count = result['source']['rules']['fn_length']
        self.assertEqual(count['invalid_suppression'], 1)
        self.assertEqual(count['source_policy_unavailable'], 0)

    def test_function_context_evidence_must_match_exactly(self):
        for field, value in (('compilation_unit', 'other:sample:test=false'),
                             ('unit_evidence', {'report_unit': 0, 'function': 2})):
            report, labels = fixture()
            report['functions'][0][field] = value
            with self.assertRaisesRegex(ValueError, 'context mismatch|association mismatch'):
                summarize(report, labels, ROOT)

    def test_unrequested_sample_does_not_block_unrelated_ambiguous_names(self):
        report, labels = fixture()
        owner = deepcopy(report['functions'][0])
        owner['key'] = 'function:u0:f1'
        owner['unit_evidence']['function'] = 1
        report['functions'].append(owner)
        row = deepcopy(report['analysis']['functions'][0])
        row['key'] = owner['key']
        report['analysis']['functions'].append(row)
        report['analysis']['scope'].update(expected_functions=2, assessed_functions=2, evaluated_rules=6)
        report['mutation_effects']['expected_functions'] = 2
        labels['samples'][0]['function'] = 'another::f'
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['quality']['counts'], {'unobserved': 1})

    def test_density_lifecycle_preserves_budget_and_unknown_distinctions(self):
        report, labels = fixture()
        effects = report['mutation_effects']
        row = effects['functions'][0]
        row['evidence'].update(execution='budget_exhausted', coverage='incomplete',
                               availability={'status': 'unknown', 'value': {'reason': 'budget'}})
        row['density'].update(complete=False)
        row['density']['measurement'].update(outcome='not_evaluated', actual=None)
        row['density']['event_inventory']['coverage'] = 'incomplete'
        effects.update(assessed_functions=0, evaluated_functions=0, unavailable_functions=1,
                       execution='budget_exhausted', coverage='incomplete')
        result = summarize(report, labels, ROOT)
        density = result['density']
        self.assertEqual(density['lifecycle']['effect_execution'], {'budget_exhausted': 1})
        self.assertEqual(density['lifecycle']['effect_availability'], {'unknown': 1})
        self.assertFalse(density['functions'][0]['assessed'])
        self.assertEqual(density['aggregates']['unavailable_functions'], 1)

    def test_density_aggregate_and_visibility_contradictions_fail(self):
        for field in ('assessed_functions', 'evaluated_functions', 'unavailable_functions', 'findings', 'visible_findings'):
            report, labels = fixture()
            report['mutation_effects'][field] += 1
            with self.assertRaisesRegex(ValueError, 'density aggregate'):
                summarize(report, labels, ROOT)

    def test_extra_guard_consumer_is_rejected(self):
        report, labels = fixture()
        report['scope']['build_options']['guard_clauses'] = True
        with self.assertRaisesRegex(ValueError, 'guard analysis'):
            summarize(report, labels, ROOT)

    def test_disabled_spacing_is_distinct_from_known_measurement(self):
        report, labels = fixture()
        row = report['analysis']['functions'][0]
        spacing = next(item for item in row['assessments'] if item['rule_id'] == 'fn_spacing')
        spacing.update(availability='not_applicable', actual=None, limit=0)
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['source']['rules']['fn_spacing']['disabled'], 1)
        self.assertEqual(result['source']['rules']['fn_spacing']['known_evaluated'], 0)

    def test_density_invalid_suppression_is_a_separate_finding(self):
        report, labels = fixture()
        effects = report['mutation_effects']
        value = effects['functions'][0]['density']
        value['source_suppression'] = {'status': 'invalid'}
        value.update(invalid_suppression={'rule_id': 'invalid_suppression'}, findings=1, visible_findings=1)
        effects.update(findings=1, visible_findings=1)
        result = summarize(report, labels, ROOT)
        self.assertEqual(result['density']['counts']['invalid_suppression'], 1)
        self.assertEqual(result['density']['counts']['source_policy_unavailable'], 0)

    def test_duplicate_json_object_keys_fail_at_cli_load(self):
        report, labels = fixture()
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            report_path, label_path, output = [directory / name for name in ('report.json', 'labels.json', 'out.json')]
            # The contradictory duplicate is nested, where a last-key-wins parser
            # could otherwise silently turn a failed run into completed capture.
            encoded = json.dumps(report).replace('"source_rules": true',
                                                '"source_rules": false, "source_rules": true')
            report_path.write_text(encoded)
            label_path.write_text(json.dumps(labels))
            command = [sys.executable, str(Path(__file__).with_name('project_coverage.py')),
                       '--report', str(report_path), '--labels', str(label_path),
                       '--workspace-root', str(ROOT), '--output', str(output)]
            completed = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(completed.returncode, 2)
            self.assertIn('duplicate JSON object key: source_rules', completed.stderr)
            self.assertFalse(output.exists())

    def test_explicit_file_load(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'input.json'
            path.write_text(json.dumps({'ok': 1}))
            self.assertEqual(load(path), {'ok': 1})


if __name__ == '__main__':
    unittest.main()
