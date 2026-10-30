"""Offline complexity comparisons consume completed observations without rescoring."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from compiler_complexity import compare
from test_compiler_quality import fixture as quality_fixture


def fixture():
    report, labels = quality_fixture()
    labels.update(schema='rstyle.compiler-complexity-cases/1',
                  metrics='rstyle.compiler-style-metrics/6', complexity_policy_version=1,
                  limits={'max_cyclomatic': 2, 'max_cognitive': 2})
    source = 'fn f(a:bool,b:bool) { if a { if b {} } }'
    labels['samples'][0].update(test=False, source=source, unsuppressed=True,
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


def score_fixture():
    report, labels = fixture()
    source = 'fn f() {}'
    labels['samples'][0].update(source=source, source_line=1, source_column=1,
        expected={'available': True, 'policy_version': 1, 'cyclomatic': 1, 'cognitive': 0})
    report['owned_source_inventory'][0]['source_functions'][0]['source'] = source
    advisory = report['compiler_advisory']
    advisory['planning']['groups'] = []
    unit = advisory['units'][0]
    unit['evaluation'] = {'diagnostics': [], 'summary': {
        'violations': 0, 'functions': 1, 'complexity_measured': 1, 'complexity_unavailable': 0}}
    unit['complexity_ranking'] = {
        'schema': 'rstyle.compiler-complexity-ranking/1', 'requested_limit': 1,
        'known_functions': 1, 'measured_functions': 1,
        'unavailable_measurements': 0, 'unavailable_owners': 0, 'input_complete': True,
        'owners': ['function:u0:f0'],
        'functions': [{'id': 'f', 'name': 'f', 'path': 'src/lib.rs', 'line': 1, 'column': 1,
                       'complexity': {'policy_version': 1, 'cyclomatic': 1, 'cognitive': 0}}]}
    return report, labels


def score_rank(report):
    return report['compiler_advisory']['units'][0]['complexity_ranking']


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
        unit['evaluation'] = {'diagnostics': [], 'summary': {'violations': 0, 'functions': 1}}
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

    def test_threshold_comparison_requires_explicit_review_of_unsuppressed_source(self):
        for precondition in ('missing', False, None, 1):
            for require_scores in (False, True):
                with self.subTest(precondition=precondition, require_scores=require_scores):
                    report, labels = score_fixture()
                    if precondition == 'missing':
                        del labels['samples'][0]['unsuppressed']
                    else:
                        labels['samples'][0]['unsuppressed'] = precondition
                    self.assertIsNotNone(score_rank(report)['functions'][0]['complexity'])
                    with self.assertRaisesRegex(ValueError, 'suppression|unsuppressed'):
                        compare(report, labels, require_scores=require_scores)

    def test_reviewed_unsuppressed_clean_source_preserves_exact_score_comparison(self):
        report, labels = score_fixture()
        self.assertIs(labels['samples'][0]['unsuppressed'], True)
        result = compare(report, labels, require_scores=True)
        self.assertEqual(result['counts'], {'agreement': 2})
        self.assertEqual(result['score_counts'], {'agreement': 1})

    def test_clean_comparisons_do_not_invent_numeric_measurements(self):
        report, labels = fixture()
        report['compiler_advisory']['units'][0]['evaluation'] = {
            'diagnostics': [], 'summary': {'violations': 0, 'functions': 1}}
        report['compiler_advisory']['planning']['groups'] = []
        labels['samples'][0]['expected'].update(cyclomatic=1, cognitive=0)
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2})
        for row in result['comparisons']:
            self.assertEqual(row['observed'], {'outcome': 'evaluated_no_finding'})


class CompilerComplexityExclusionTests(unittest.TestCase):
    def sample(self, pattern, path, excluded):
        report, labels = score_fixture()
        labels['samples'][0]['source_file'] = path
        report['owned_source_inventory'][0]['source_functions'][0]['source_file'] = '/workspace/' + path
        report['compiler_advisory']['effective_config']['exclude'] = [pattern]
        unit = report['compiler_advisory']['units'][0]
        unit['evaluation']['summary']['functions'] = int(not excluded)
        ranking = score_rank(report)
        if excluded:
            unit['evaluation']['summary']['complexity_measured'] = 0
            ranking.update(known_functions=0, measured_functions=0, owners=[], functions=[])
        else:
            ranking['functions'][0]['path'] = path
        return report, labels

    def assert_outcome(self, pattern, path, excluded):
        report, labels = self.sample(pattern, path, excluded)
        for require_scores in (False, True):
            result = compare(report, labels, require_scores=require_scores)
            self.assertEqual(result['counts'], {'unavailable' if excluded else 'agreement': 2})
            for row in result['comparisons']:
                if excluded:
                    self.assertIsNone(row['observed'])
                    self.assertTrue(row['reason'])
                else:
                    self.assertEqual(row['observed'], {'outcome': 'evaluated_no_finding'})
            if require_scores:
                self.assertEqual(result['score_counts'],
                                 {'unavailable' if excluded else 'agreement': 1})
                if excluded:
                    self.assertIsNone(result['score_comparisons'][0]['observed'])

    def test_cli_refuses_clean_success_for_a_policy_excluded_sample(self):
        report, labels = self.sample('src/**', 'src/lib.rs', True)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, document in [('report', report), ('labels', labels)]:
                (root / (name + '.json')).write_text(json.dumps(document))
            command = [sys.executable, str(Path(__file__).with_name('compiler_complexity.py')),
                       '--report', str(root / 'report.json'), '--labels', str(root / 'labels.json'),
                       '--output', str(root / 'result.json')]
            output = subprocess.run(command, capture_output=True)
            self.assertEqual(output.returncode, 2, output.stderr)
            result = json.loads((root / 'result.json').read_text())
            self.assertEqual(result['counts'], {'unavailable': 2})
            self.assertTrue(all(row['observed'] is None for row in result['comparisons']))

    def test_captured_but_policy_excluded_owner_is_not_an_evaluated_clean_negative(self):
        self.assert_outcome('src/lib.rs', 'src/lib.rs', True)

    def test_exact_exclusion_does_not_exclude_a_longer_filename(self):
        self.assert_outcome('src/lib.rs', 'src/lib.rs.bak', False)

    def test_root_recursive_exclusions_preserve_member_source_distinction(self):
        for path, excluded in [('src/nested/lib.rs', True),
                               ('crates/member/src/lib.rs', False)]:
            with self.subTest(path=path):
                self.assert_outcome('src/**', path, excluded)

    def test_explicit_recursive_source_glob_matches_root_and_members_only(self):
        for path, excluded in [('src/lib.rs', True), ('crates/member/src/lib.rs', True),
                               ('crates/member/source/lib.rs', False)]:
            with self.subTest(path=path):
                self.assert_outcome('**/src/**', path, excluded)


class CompilerComplexityScoreTests(unittest.TestCase):
    def test_ranked_clean_owner_remains_evaluated_when_other_captured_owners_are_excluded(self):
        report, labels = score_fixture()
        unit = report['compiler_advisory']['units'][0]
        unit['observations'].append({'key': 'function:u0:f1', 'name': 'excluded'})
        unit['evaluation']['summary']['functions'] = 1
        for required in (False, True):
            result = compare(report, labels, require_scores=required)
            self.assertEqual(result['counts'], {'agreement': 2})
            if required:
                self.assertEqual(result['score_counts'], {'agreement': 1})
        del unit['complexity_ranking']
        self.assertEqual(compare(report, labels)['counts'], {'unavailable': 2})

    def test_opt_in_preserves_default_comparisons_and_adds_one_exact_score_row(self):
        report, labels = score_fixture()
        default = compare(report, labels)
        self.assertNotIn('score_counts', default)
        self.assertNotIn('score_comparisons', default)
        absent = deepcopy(report)
        absent['compiler_advisory']['units'][0].pop('complexity_ranking')
        self.assertEqual(compare(absent, labels), default)
        result = compare(report, labels, require_scores=True)
        scores = result.pop('score_comparisons')
        self.assertEqual(result.pop('score_counts'), {'agreement': 1})
        self.assertIn('explicitly requested completed ranked scores', result.pop('limitation'))
        default.pop('limitation')
        self.assertEqual(result, default)
        self.assertEqual(len(scores), 1)
        row = scores[0]
        self.assertEqual(row['expected'], {'policy_version': 1, 'cyclomatic': 1, 'cognitive': 0})
        self.assertEqual(row['observed'], row['expected'])
        self.assertEqual((row['function'], row['source_file'], row['test'], row['key']),
                         ('f', 'src/lib.rs', False, 'function:u0:f0'))
        self.assertEqual(row['result'], 'agreement')

    def test_clean_diagnostics_cannot_hide_wrong_numeric_scores(self):
        for field, value in [('cyclomatic', 2), ('cognitive', 1)]:
            with self.subTest(field=field):
                report, labels = score_fixture()
                score_rank(report)['functions'][0]['complexity'][field] = value
                result = compare(report, labels, require_scores=True)
                self.assertEqual(result['counts'], {'agreement': 2})
                self.assertEqual(result['score_counts'], {'disagreement': 1})
                self.assertEqual(result['score_comparisons'][0]['observed'][field], value)

    def test_score_policy_version_is_compared_independently(self):
        report, labels = score_fixture()
        score_rank(report)['functions'][0]['complexity']['policy_version'] = 2
        result = compare(report, labels, require_scores=True)
        self.assertEqual(result['counts'], {'agreement': 2})
        self.assertEqual(result['score_counts'], {'disagreement': 1})
        self.assertEqual(result['score_comparisons'][0]['observed']['policy_version'], 2)

    def test_missing_or_null_rank_scores_stay_unavailable_without_zero_defaults(self):
        for missing in ('ranking', 'score'):
            with self.subTest(missing=missing):
                report, labels = score_fixture()
                if missing == 'ranking':
                    del report['compiler_advisory']['units'][0]['complexity_ranking']
                else:
                    ranking = score_rank(report)
                    ranking.update(measured_functions=0, unavailable_measurements=1, input_complete=False)
                    ranking['functions'][0]['complexity'] = None
                    report['compiler_advisory']['units'][0]['evaluation']['summary'].update(
                        complexity_measured=0, complexity_unavailable=1)
                result = compare(report, labels, require_scores=True)
                self.assertEqual(result['counts'], {'agreement': 2})
                self.assertEqual(result['score_counts'], {'unavailable': 1})
                self.assertIsNone(result['score_comparisons'][0]['observed'])
                self.assertTrue(result['score_comparisons'][0]['reason'])

    def test_top_limit_truncation_cannot_turn_unlisted_sample_into_zero(self):
        report, labels = score_fixture()
        other = deepcopy(report['functions'][0])
        other.update(key='function:u0:f1', name='g', fn_span={'start': 100, 'end': 108})
        report['functions'].append(other)
        unit = report['compiler_advisory']['units'][0]
        unit['expected_functions'] = 2
        unit['observations'].append({'key': 'function:u0:f1', 'name': 'g'})
        ranking = score_rank(report)
        ranking.update(known_functions=2, measured_functions=2)
        unit['evaluation']['summary'].update(functions=2, complexity_measured=2)
        ranking['owners'] = ['function:u0:f1']
        ranking['functions'][0].update(id='g', name='g', line=2)
        ranking['functions'][0]['complexity'].update(cyclomatic=2, cognitive=1)
        result = compare(report, labels, require_scores=True)
        self.assertEqual(result['score_counts'], {'unavailable': 1})
        self.assertIsNone(result['score_comparisons'][0]['observed'])

    def test_present_scores_do_not_override_original_source_or_context_failure(self):
        for changes in ({'source': 'different authored source'}, {'source_file': 'src/else.rs'},
                        {'test': True}, {'target_name': 'other'}):
            with self.subTest(changes=changes):
                report, labels = score_fixture()
                labels['samples'][0].update(changes)
                result = compare(report, labels, require_scores=True)
                self.assertEqual(result['score_counts'], {'unavailable': 1})
                self.assertIsNone(result['score_comparisons'][0]['observed'])

    def test_rank_rejects_wrong_duplicate_or_out_of_unit_owners(self):
        for owners in (['function:u0:f99'], ['function:u1:f0'],
                       ['function:u0:f0', 'function:u0:f0']):
            with self.subTest(owners=owners):
                report, labels = score_fixture()
                ranking = score_rank(report)
                ranking['owners'] = owners
                if len(owners) == 2:
                    ranking['requested_limit'] = 2
                    ranking['functions'].append(deepcopy(ranking['functions'][0]))
                with self.assertRaises(ValueError):
                    compare(report, labels, require_scores=True)

    def test_rank_rejects_source_identity_mismatches_and_unpaired_rosters(self):
        for change in ({'name': 'other'}, {'path': 'src/else.rs'},
                       {'line': 0}, {'column': 0}, None):
            with self.subTest(change=change):
                report, labels = score_fixture()
                if change is None:
                    score_rank(report)['owners'] = []
                else:
                    score_rank(report)['functions'][0].update(change)
                with self.assertRaises(ValueError):
                    compare(report, labels, require_scores=True)

    def test_source_annotation_locations_do_not_replace_exact_byte_owner_association(self):
        report, labels = score_fixture()
        # Source labels can locate leading attributes/comments; measured rows
        # locate the function. sample_owner still checks the exact source/span.
        score_rank(report)['functions'][0].update(line=3, column=5)
        result = compare(report, labels, require_scores=True)
        self.assertEqual(result['score_counts'], {'agreement': 1})

    def test_rank_rejects_malformed_schema_limits_and_measurement_counts(self):
        for change in ({'schema': 'unknown'}, {'requested_limit': 0}, {'requested_limit': True},
                       {'known_functions': -1}, {'known_functions': 2}, {'measured_functions': 0},
                       {'unavailable_measurements': 1}, {'unavailable_owners': -1},
                       {'input_complete': False}, {'input_complete': 1}):
            with self.subTest(change=change):
                report, labels = score_fixture()
                score_rank(report).update(change)
                with self.assertRaises(ValueError):
                    compare(report, labels, require_scores=True)

    def test_rank_rejects_non_numeric_or_missing_score_fields(self):
        for field, value in [('cyclomatic', True), ('cognitive', -1),
                             ('policy_version', '1'), ('cyclomatic', None)]:
            with self.subTest(field=field, value=value):
                report, labels = score_fixture()
                score_rank(report)['functions'][0]['complexity'][field] = value
                with self.assertRaises(ValueError):
                    compare(report, labels, require_scores=True)
        report, labels = score_fixture()
        del score_rank(report)['functions'][0]['complexity']['cognitive']
        with self.assertRaises(ValueError):
            compare(report, labels, require_scores=True)

    def test_cli_score_mismatch_and_unavailability_exit_two_while_default_stays_clean(self):
        for missing in (False, True):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as directory:
                report, labels = score_fixture()
                if missing:
                    del report['compiler_advisory']['units'][0]['complexity_ranking']
                else:
                    score_rank(report)['functions'][0]['complexity']['cognitive'] = 1
                root = Path(directory)
                for name, document in [('report', report), ('labels', labels)]:
                    (root / (name + '.json')).write_text(json.dumps(document))
                command = [sys.executable, str(Path(__file__).with_name('compiler_complexity.py')),
                           '--report', str(root / 'report.json'), '--labels', str(root / 'labels.json'),
                           '--output', str(root / 'default.json')]
                default = subprocess.run(command, capture_output=True)
                self.assertEqual(default.returncode, 0, default.stderr)
                command[-1] = str(root / 'scores.json')
                scored = subprocess.run(command + ['--require-scores'], capture_output=True)
                self.assertEqual(scored.returncode, 2, scored.stderr)
                result = json.loads((root / 'scores.json').read_text())
                self.assertEqual(result['counts'], {'agreement': 2})
                self.assertEqual(result['score_counts'],
                                 {'unavailable' if missing else 'disagreement': 1})


if __name__ == '__main__':
    unittest.main()
