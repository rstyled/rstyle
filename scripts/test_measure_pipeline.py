"""Exact matching-score parity checks used by controlled timing runs."""
import copy
import unittest

from measure_library_resolution import exact_scores


class ExactScoreParityTests(unittest.TestCase):
    def report(self):
        return {
            'functions': [{'key': 'a', 'facts': {'name': 'first'}},
                          {'key': 'b', 'facts': {'name': 'second'}}],
            'matching': {'pairs': [{
                'first': 'a', 'second': 'b', 'displayed_score': '0.5',
                'stages': [{'stage': 'Wl', 'detail': {
                    'score': '1/2', 'norm_a_squared': '4', 'norm_b_squared': '9',
                    'overlap_per_round': '[2, 1]', 'self_a_per_round': '[4, 3]',
                    'self_b_per_round': '[9, 7]'}}]}]},
        }

    def test_exact_score_and_round_changes_are_detected(self):
        report = self.report()
        for key, value in [('score', '50001/100000'), ('overlap_per_round', '[2, 2]')]:
            changed = copy.deepcopy(report)
            changed['matching']['pairs'][0]['stages'][0]['detail'][key] = value
            self.assertNotEqual(exact_scores(report), exact_scores(changed))

    def test_pair_orientation_does_not_change_named_score_identity(self):
        report = self.report()
        changed = copy.deepcopy(report)
        pair = changed['matching']['pairs'][0]
        pair['first'], pair['second'] = pair['second'], pair['first']
        detail = pair['stages'][0]['detail']
        for first, second in [('norm_a_squared', 'norm_b_squared'),
                              ('self_a_per_round', 'self_b_per_round')]:
            detail[first], detail[second] = detail[second], detail[first]
        self.assertEqual(exact_scores(report), exact_scores(changed))


if __name__ == '__main__':
    unittest.main()
