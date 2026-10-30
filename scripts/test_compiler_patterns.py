import unittest
from compiler_patterns import evaluation


class CompilerPatterns(unittest.TestCase):
    def test_reads_shared_workspace_result_without_duplicating_units(self):
        raw = {'candidates': [{'id': 'candidate'}], 'signature_groups': [], 'suppressed': []}
        report = {'capture_complete': True, 'assessment_complete': True,
                  'compiler_advisory': {'units': [
                      {'patterns': {'input_complete': True, 'evaluation': raw}},
                      {'patterns': {'input_complete': True, 'evaluation': raw}},
                  ]}}
        self.assertIs(evaluation(report), raw)
        report['compiler_advisory']['units'][1]['patterns']['input_complete'] = False
        with self.assertRaises(ValueError):
            evaluation(report)
        report['compiler_advisory']['units'] = []
        with self.assertRaises(ValueError):
            evaluation(report)
        report['capture_complete'] = False
        with self.assertRaises(ValueError):
            evaluation(report)

    def test_retained_flat_reports_remain_readable_but_missing_is_not_empty(self):
        raw = {'candidates': [], 'signature_groups': [], 'suppressed': []}
        self.assertIs(evaluation(raw), raw)
        with self.assertRaises(ValueError):
            evaluation({})
