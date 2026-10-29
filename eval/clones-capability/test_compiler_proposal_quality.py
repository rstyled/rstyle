"""Independent source-role labels for completed compiler proposal reviews."""
from copy import deepcopy
import unittest

from compiler_proposal_quality import compare
from test_compiler_quality import fixture as historical_fixture


def fixture():
    report, _ = historical_fixture()
    unit = report['compiler_advisory']['units'][0]
    unit['observations'] = []
    report['functions'] = []
    report['owned_source_inventory'][0]['source_functions'] = []
    owners = []
    for index, name in enumerate(('f', 'g', 'helper')):
        source = f'fn {name}(value: u32) -> u32 {{ (value + 1) * 2 }}'
        span = {'start': index * 100, 'end': index * 100 + len(source)}
        key = f'function:u0:f{index}'
        compiler_id = f'sample::{name}'
        unit['observations'].append({'key': key, 'name': name})
        report['functions'].append({'key': key, 'name': name, 'fn_span': span,
                                    'unit_evidence': {'report_unit': 0}})
        report['owned_source_inventory'][0]['source_functions'].append({
            'function_span': span, 'active_functions': [name],
            'source_file': '/workspace/src/lib.rs', 'source': source,
        })
        owners.append({'package': 'sample', 'target_kind': 'lib', 'function': name,
                       'source_file': 'src/lib.rs', 'source': source,
                       'included': True, 'unsuppressed': True, 'compiler_id': compiler_id})
    policy = {
        'threshold_milli': 600, 'min_coverage_milli': 600, 'pair_coverage_milli': None,
        'single_call_guard': False, 'max_holes': 3,
        'normalize': {name: False for name in ('ctor', 'pred', 'case', 'exit', 'counter', 'cmp', 'propagate')},
    }

    def site(owner, line):
        return {'path': owner['source_file'], 'line': line, 'id': owner['compiler_id'],
                'name': owner['function'], 'locator': owner['compiler_id'], 'shape': 'opaque-existing-shape'}

    candidate = {'id': 'opaque-candidate', 'kind': 'fn_param', 'score_milli': 700,
                 'breakdown': {'support': 2, 'lift_milli': 1000, 'holes': 1, 'coverage_milli': 900},
                 'observation': 'Similar computations with a callable difference.',
                 'inference': 'A shared helper might accept the callable.',
                 'possible_refactor': 'Extract after checking the source roles.',
                 'counter_evidence': [],
                 'sites': [site(owners[0], 1), site(owners[1], 2)],
                 'definitions': [site(owners[2], 3)]}
    unit['patterns'] = {
        'schema': 'rstyle.compiler-patterns/1', 'policy': deepcopy(policy),
        'input_complete': False, 'policy_selected_functions': 3, 'observed_graphs': 3,
        'unavailable_functions': 0, 'unavailable_graphs': [],
        'evaluation': {'signature_groups': [], 'candidates': [candidate], 'suppressed': [],
                       'ignored': [], 'ignored_count': 0},
    }
    labels = {'schema': 'rstyle.compiler-proposal-quality-cases/1',
              'run': 'run-sample', 'toolchain': 'nightly', 'metrics': 'metric/1',
              'policy': deepcopy(policy),
              'samples': [{'id': 'source-reviewed-family', 'kind': 'fn_param',
                           'sites': owners[:2], 'definitions': owners[2:],
                           'judgment': 'useful',
                           'rationale': 'The two source operations share a useful callable seam.'}]}
    return report, labels


class CompilerProposalQualityTests(unittest.TestCase):
    def test_exact_roles_agree_without_completing_scope(self):
        report, labels = fixture()
        result = compare(report, labels)
        self.assertEqual(result['schema'], 'rstyle.compiler-proposal-quality-observation/1')
        self.assertEqual(result['counts'], {'structural_agreement': 1})
        self.assertTrue(result['sample_complete'])
        self.assertFalse(result['scope_complete'])
        self.assertEqual(result['samples'][0]['judgment'], 'useful')
        self.assertEqual(result['inventory'][0]['sampled_by'], ['source-reviewed-family'])

    def test_usefulness_is_independent_of_structural_agreement(self):
        for judgment in ('useful', 'needs_revision', 'not_useful'):
            report, labels = fixture()
            labels['samples'][0]['judgment'] = judgment
            result = compare(report, labels)
            self.assertEqual(result['counts'], {'structural_agreement': 1})
            self.assertEqual(result['samples'][0]['judgment'], judgment)

    def test_candidate_id_and_site_order_are_not_matching_authority(self):
        report, labels = fixture()
        candidate = report['compiler_advisory']['units'][0]['patterns']['evaluation']['candidates'][0]
        candidate['id'] = 'different-opaque-id'
        candidate['sites'].reverse()
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'structural_agreement': 1})
        self.assertEqual(result['inventory'][0]['id'], 'different-opaque-id')

    def test_changed_family_or_member_roles_do_not_agree(self):
        for changed in ('kind', 'roles'):
            report, labels = fixture()
            candidate = report['compiler_advisory']['units'][0]['patterns']['evaluation']['candidates'][0]
            if changed == 'kind':
                candidate['kind'] = 'parameterize'
            else:
                candidate['sites'], candidate['definitions'] = candidate['definitions'], candidate['sites']
            result = compare(report, labels)
            self.assertEqual(result['counts'], {'unavailable': 1})
            self.assertNotIn('judgment', result['samples'][0])

    def test_absent_candidate_is_not_observed_only_for_complete_input(self):
        for complete in (False, True):
            report, labels = fixture()
            patterns = report['compiler_advisory']['units'][0]['patterns']
            patterns['evaluation']['candidates'] = []
            patterns['input_complete'] = complete
            result = compare(report, labels)
            expected = 'not_observed' if complete else 'unavailable'
            self.assertEqual(result['counts'], {expected: 1})
            self.assertNotIn('judgment', result['samples'][0])

    def test_unsampled_candidates_are_explicitly_unreviewed(self):
        report, labels = fixture()
        candidates = report['compiler_advisory']['units'][0]['patterns']['evaluation']['candidates']
        extra = deepcopy(candidates[0])
        extra.update(id='other-proposal', kind='generic_fn')
        candidates.append(extra)
        result = compare(report, labels)
        self.assertEqual(len(result['inventory']), 2)
        self.assertEqual(result['inventory'][1]['sampled_by'], [])
        self.assertEqual(result['inventory'][1]['unit'], 0)
        self.assertEqual(result['inventory'][1]['index'], 1)

    def test_visibility_is_separate_from_raw_review_inventory(self):
        report, labels = fixture()
        report['compiler_advisory']['units'][0]['patterns']['visible_candidates'] = []
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'structural_agreement': 1})
        self.assertFalse(result['inventory'][0]['visible'])
        self.assertEqual(result['inventory'][0]['sampled_by'], ['source-reviewed-family'])

    def test_changed_missing_or_ambiguous_original_source_is_unavailable(self):
        for changed in ('bytes', 'missing', 'ambiguous'):
            report, labels = fixture()
            sources = report['owned_source_inventory'][0]['source_functions']
            if changed == 'bytes':
                sources[0]['source'] += '\n'
            elif changed == 'missing':
                sources.pop(0)
            else:
                sources.append(deepcopy(sources[0]))
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_missing_graph_owner_stays_unavailable(self):
        report, labels = fixture()
        patterns = report['compiler_advisory']['units'][0]['patterns']
        patterns['unavailable_graphs'] = [{'key': 'function:u0:f0', 'reason': 'graph not captured'}]
        patterns['observed_graphs'] = 2
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_foreign_missing_graph_key_cannot_establish_agreement(self):
        report, labels = fixture()
        patterns = report['compiler_advisory']['units'][0]['patterns']
        patterns['unavailable_graphs'] = [{'key': 'foreign-owner', 'reason': 'not captured'}]
        patterns['observed_graphs'] = 2
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_unknown_selected_graph_population_cannot_establish_agreement(self):
        for field, value in (('policy_selected_functions', 4), ('observed_graphs', 2)):
            report, labels = fixture()
            report['compiler_advisory']['units'][0]['patterns'][field] = value
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_missing_or_ambiguous_observed_owner_is_unavailable(self):
        for missing in (True, False):
            report, labels = fixture()
            if missing:
                unit = report['compiler_advisory']['units'][0]
                owner = unit['observations'].pop(0)
                unit['unavailable'].append({'key': owner['key'], 'reason': 'source owner not captured'})
                unit['patterns']['policy_selected_functions'] = 2
                unit['patterns']['observed_graphs'] = 2
            else:
                report['functions'].append(deepcopy(report['functions'][0]))
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_candidate_name_and_path_must_match_the_source_owner(self):
        for field, value in (('name', 'other'), ('path', 'src/other.rs')):
            report, labels = fixture()
            report['compiler_advisory']['units'][0]['patterns']['evaluation']['candidates'][0]['sites'][0][field] = value
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 1})

    def test_duplicate_label_ids_and_candidate_role_identity_are_rejected(self):
        for duplicate in ('sample', 'candidate'):
            report, labels = fixture()
            if duplicate == 'sample':
                labels['samples'].append(deepcopy(labels['samples'][0]))
            else:
                candidates = report['compiler_advisory']['units'][0]['patterns']['evaluation']['candidates']
                extra = deepcopy(candidates[0])
                extra['id'] = 'different-id-same-role-family'
                candidates.append(extra)
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_wrong_context_metric_schema_and_policy_reject(self):
        for changed in ('run', 'toolchain', 'metrics', 'schema', 'policy'):
            report, labels = fixture()
            if changed == 'run':
                labels['run'] = 'other-run'
            elif changed in ('toolchain', 'metrics', 'schema'):
                labels[changed] = 'other'
            else:
                report['compiler_advisory']['units'][0]['patterns']['policy']['max_holes'] = 4
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_unreviewed_source_or_invalid_usefulness_label_rejects(self):
        for changed in ('included', 'unsuppressed', 'judgment', 'rationale'):
            report, labels = fixture()
            if changed in ('included', 'unsuppressed'):
                labels['samples'][0]['sites'][0][changed] = False
            elif changed == 'judgment':
                labels['samples'][0]['judgment'] = 'automatically_safe'
            else:
                labels['samples'][0]['rationale'] = ''
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_review_does_not_mutate_report_or_source_labels(self):
        report, labels = fixture()
        original_report, original_labels = deepcopy(report), deepcopy(labels)
        compare(report, labels)
        self.assertEqual(report, original_report)
        self.assertEqual(labels, original_labels)


if __name__ == '__main__':
    unittest.main()
