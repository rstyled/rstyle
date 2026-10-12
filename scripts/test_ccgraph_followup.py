"""Source-specified follow-up oracles and bookkeeping expectations, not detector evidence."""
import copy
import unittest
from unittest.mock import patch
import ccgraph_followup as f
import ccgraph_study as b

class Followup(unittest.TestCase):
    def test_frozen_inputs(self):
        frozen = f.validate()
        self.assertEqual(frozen['version'], 2)
        manifest = b.read(f.STUDY / 'manifest.json')
        self.assertEqual(len(manifest['functions']), 15)
        self.assertEqual(len(manifest['labels']), 21)

    def test_source_oracle(self):
        oracle = {'counts': {'formal_input': 1, 'read': 1, 'return': 1}, 'operators': ['&']}
        document = {'nodes': [{'category': c, 'attributes': {}} for c in ['formal_input', 'read', 'return']] +
                    [{'category': 'computation', 'attributes': {'operator': {'status': 'known', 'value': '&'}}}]}
        self.assertTrue(f.compare_oracle(document, oracle)['agrees'])
        document['nodes'].reverse()
        self.assertTrue(f.compare_oracle(document, oracle)['agrees'])
        changed = copy.deepcopy(document); changed['nodes'][0]['attributes']['operator']['value'] = '|'
        self.assertFalse(f.compare_oracle(changed, oracle)['agrees'])
        document['nodes'].append({'category': 'read', 'attributes': {}})
        self.assertFalse(f.compare_oracle(document, oracle)['agrees'])

    def test_primary_target_minima_and_undefined(self):
        self.assertEqual(f.target(0, 0, 3, 0, 2)['value'], None)
        self.assertEqual(f.target(2, 2, 3, 2, 2)['outcome'], 'insufficient evidence')
        self.assertEqual(f.target(3, 3, 3, 1, 2)['outcome'], 'insufficient evidence')
        self.assertEqual(f.target(2, 3, 3, 2, 2)['outcome'], 'not met')
        self.assertEqual(f.target(3, 3, 3, 2, 2)['outcome'], 'met')

    def test_secondary_frozen_targets(self):
        m = b.metrics([])
        self.assertTrue(all(t['outcome'] == 'insufficient evidence' for t in f.quality_targets(m).values()))
        m.update(evaluable_positive=20, evaluable_negative=20, independent_groups=['a', 'b'],
                 precision=.94, recall=.80, false_positive_rate=.05, f1=.85,
                 positive_admission_coverage=.80, end_to_end_positive_yield=.70)
        self.assertEqual(f.quality_targets(m)['precision']['outcome'], 'not met')
        self.assertEqual(f.quality_targets(m)['recall']['outcome'], 'met')
        self.assertEqual(f.quality_targets(m)['f1']['outcome'], 'met')

    def test_coverage_loss_is_correctness_failure(self):
        rows = [dict(id=str(i), test=False, expected='admitted', actual='admitted', outcome_agrees=True, oracle={'agrees': True}) for i in range(3)]
        results = [{'scope': {'id': 'p1', 'population': 'natural', 'split': 'heldout'}, 'variant': 'default', 'correctness': rows[:2]},
                   {'scope': {'id': 'p2', 'population': 'natural', 'split': 'heldout'}, 'variant': 'default', 'correctness': rows[2:]}]
        self.assertEqual(f.coverage_targets(results)['natural_heldout_admission']['outcome'], 'met')
        rows[0].update(actual='excluded', outcome_agrees=False, oracle=None)
        self.assertEqual(f.coverage_targets(results)['natural_heldout_admission']['denominator'], 3)
        self.assertEqual(f.coverage_targets(results)['natural_heldout_admission']['outcome'], 'not met')
        self.assertEqual(f.coverage_targets(results)['natural_heldout_bounded_oracles']['numerator'], 2)

    def test_target_aggregation_ignores_order_and_test_contexts(self):
        def row(i, test=False):
            return dict(id=str(i), test=test, expected='admitted', actual='admitted', outcome_agrees=True, oracle={'agrees': True})
        results = [dict(scope=dict(id='p1', population='natural', split='heldout'), variant='default', correctness=[row(1), row(2), row(1, True), row(2, True)]),
                   dict(scope=dict(id='p2', population='natural', split='heldout'), variant='default', correctness=[row(3), row(3, True)])]
        expected = f.coverage_targets(results)
        self.assertEqual(expected['natural_heldout_admission']['denominator'], 3)
        results.reverse()
        for r in results: r['correctness'].reverse()
        self.assertEqual(f.coverage_targets(results), expected)

    def test_natural_histories_stay_in_one_split(self):
        manifest = b.read(f.STUDY / 'manifest.json')
        scopes = {s['id']: s for s in manifest['scopes']}
        groups = {}
        for function in manifest['functions']:
            groups.setdefault(function['provenance_group'], set()).add(scopes[function['scope']]['split'])
        self.assertEqual(groups, {'lz4': {'development'}, 'lexical': {'development'}, 'audit': {'development'},
                                  'regex': {'heldout'}, 'time': {'heldout'}})

if __name__ == '__main__':
    unittest.main()
