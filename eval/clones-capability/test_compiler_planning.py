"""Check source association and completed-plan comparisons without running a planner."""
from copy import deepcopy
import unittest

from compiler_planning import compare
from test_compiler_quality import fixture as quality_fixture


def fixture():
    report, _ = quality_fixture()
    target = report['observed_units'][0]['target']
    target['name'] = 'sample'
    advisory = report['compiler_advisory']
    advisory['metrics'] = 'rstyle.compiler-style-metrics/6'
    policy = {'fn_params': {'max_params': 2, 'count_self': False},
              'grouping': {'caller_rules': ['fn_params'], 'caller_hops': 1}}
    advisory['effective_config'].update(policy)
    unit = advisory['units'][0]
    unit['observations'] = []
    unit['evaluation']['diagnostics'] = []
    report['functions'] = []
    inventory = report['owned_source_inventory'][0]['source_functions']
    inventory.clear()
    samples, functions = [], []
    for i, name in enumerate(('callee', 'caller', 'clean')):
        key = 'function:u0:f' + str(i)
        span = {'start': i * 20, 'end': i * 20 + 8}
        source = 'fn ' + name + '() {}'
        unit['observations'].append({'key': key, 'name': name})
        report['functions'].append({'key': key, 'name': name, 'fn_span': span,
                                   'unit_evidence': {'report_unit': 0}})
        inventory.append({'function_span': span, 'active_functions': [name],
                          'source_file': '/workspace/src/lib.rs', 'source': source})
        samples.append({'function': name, 'source_file': 'src/lib.rs', 'source': source, 'unsuppressed': True,
                        'expected': {'params': 3 if i < 2 else 1, 'fn_params_finding': i < 2}})
        if i < 2:
            functions.append({'name': name, 'path': 'src/lib.rs', 'line': i + 1,
                              'rules': ['fn_params']})
            unit['evaluation']['diagnostics'].append({'path': 'src/lib.rs', 'line': i + 1,
                'rule_id': 'fn_params', 'actual': 3, 'limit': 2})
    unit['evaluation']['summary'].update(violations=2, functions=3)
    keys = [row['key'] for row in unit['observations']]
    bundle = [{'name': 'a', 'type': '&str'}]
    group = {'members': keys[:2], 'step_owners': keys[:2], 'bundle_owners': [keys[:2]],
             'evaluation': {'functions': functions, 'plan': {
                 'steps': [{'name': 'callee', 'signature': True},
                           {'name': 'caller', 'signature': True, 'pass_through': ['a']}],
                 'bundles': [{'params': bundle, 'functions': ['callee', 'caller']}]}}}
    advisory['planning']['groups'] = [group, {'members': [], 'step_owners': [],
                                            'bundle_owners': [], 'evaluation': {'functions': []}}]
    labels = {'schema': 'rstyle.compiler-parameter-plan-cases/1', 'run': 'run-sample',
              'toolchain': 'nightly', 'metrics': advisory['metrics'],
              'planning_schema': advisory['planning']['schema'], 'policy': deepcopy(policy),
              'package': 'sample', 'target_name': 'sample', 'target_kind': 'lib', 'test': False,
              'samples': samples, 'caller_relationship': {'caller': 'caller', 'callee': 'callee',
              'expected': {'signature_steps': ['callee', 'caller'],
                           'relative_callee_before_caller': True,
                           'caller_pass_through': ['a'], 'bundle_contains': bundle}}}
    return report, labels


class CompilerPlanningTests(unittest.TestCase):
    def test_completed_plans_agree_without_claiming_complete_scope(self):
        report, labels = fixture()
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 8})
        self.assertFalse(result['scope_complete'])
        self.assertFalse(result['planning_input_complete'])
        self.assertEqual(result['parameters'][2]['observed'], {'outcome': 'evaluated_no_finding'})

    def test_policy_excluded_clean_control_cannot_validate_parameter_plan(self):
        report, labels = fixture()
        policy = ['src/excluded.rs']
        report['compiler_advisory']['effective_config']['exclude'] = policy
        labels['policy']['exclude'] = policy
        source = report['owned_source_inventory'][0]['source_functions'][2]
        source['source_file'] = '/workspace/src/excluded.rs'
        labels['samples'][2]['source_file'] = 'src/excluded.rs'
        report['compiler_advisory']['units'][0]['evaluation']['summary']['functions'] = 2
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2, 'unavailable': 2})
        self.assertIsNone(result['parameters'][2]['observed'])
        self.assertEqual(result['relationship'][0]['result'], 'unavailable')

    def test_missing_selected_population_cannot_establish_a_clean_control(self):
        report, labels = fixture()
        del report['compiler_advisory']['units'][0]['evaluation']['summary']['functions']
        result = compare(report, labels)
        self.assertEqual(result['counts'], {'agreement': 2, 'unavailable': 2})
        self.assertIsNone(result['parameters'][2]['observed'])

    def test_suppression_precondition_requires_source_review(self):
        for value in (False, None, 1):
            report, labels = fixture()
            labels['samples'][2]['unsuppressed'] = value
            with self.assertRaisesRegex(ValueError, 'suppression precondition'):
                compare(report, labels)

    def test_missing_changed_and_ambiguous_sources_stay_unavailable(self):
        for mutation in ('changed', 'duplicate', 'missing'):
            report, labels = fixture()
            sources = report['owned_source_inventory'][0]['source_functions']
            if mutation == 'changed':
                sources[2]['source'] += ' '
            elif mutation == 'duplicate':
                sources.append(deepcopy(sources[2]))
            else:
                sources.pop()
            result = compare(report, labels)
            self.assertEqual(result['counts'], {'agreement': 2, 'unavailable': 2})

    def test_wrong_or_ambiguous_target_context_stays_unavailable(self):
        for mutation in ('name', 'test', 'duplicate'):
            report, labels = fixture()
            if mutation == 'name':
                report['observed_units'][0]['target']['name'] = 'other'
            elif mutation == 'test':
                report['observed_units'][0]['test'] = True
            else:
                report['compiler_advisory']['units'].append(
                    deepcopy(report['compiler_advisory']['units'][0]))
            self.assertEqual(compare(report, labels)['counts'], {'unavailable': 4})

    def test_malformed_parallel_owner_rosters_are_rejected(self):
        for mutation in ('steps', 'bundles', 'step_name', 'bundle_name', 'no_plan'):
            report, labels = fixture()
            group = report['compiler_advisory']['planning']['groups'][0]
            if mutation == 'steps':
                group['step_owners'].pop()
            elif mutation == 'bundles':
                group['bundle_owners'].clear()
            elif mutation == 'step_name':
                group['step_owners'][0] = 'function:u0:f2'
            elif mutation == 'bundle_name':
                group['bundle_owners'][0][0] = 'function:u0:f2'
            else:
                del group['evaluation']['plan']
            with self.assertRaises(ValueError):
                compare(report, labels)

    def test_coordinated_step_reordering_disagrees(self):
        report, labels = fixture()
        group = report['compiler_advisory']['planning']['groups'][0]
        group['step_owners'].reverse()
        group['evaluation']['plan']['steps'].reverse()
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 7, 'disagreement': 1})

    def test_wrong_forwarding_and_unrelated_bundle_owners_disagree(self):
        for mutation in ('missing', 'computed', 'owners'):
            report, labels = fixture()
            group = report['compiler_advisory']['planning']['groups'][0]
            plan = group['evaluation']['plan']
            if mutation == 'owners':
                group['bundle_owners'][0] = ['function:u0:f0', 'function:u0:f2']
                plan['bundles'][0]['functions'] = ['callee', 'clean']
            else:
                plan['steps'][1]['pass_through'] = [] if mutation == 'missing' else ['a', 'node']
            self.assertEqual(compare(report, labels)['counts'], {'agreement': 7, 'disagreement': 1})

    def test_clean_control_cannot_acquire_a_signature_step(self):
        report, labels = fixture()
        group = report['compiler_advisory']['planning']['groups'][0]
        group['step_owners'].append('function:u0:f2')
        group['evaluation']['plan']['steps'].append({'name': 'clean', 'signature': True})
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 7, 'disagreement': 1})

    def test_unavailable_clean_control_is_not_inferred_from_absent_finding(self):
        report, labels = fixture()
        unit = report['compiler_advisory']['units'][0]
        unit['observations'].pop()
        unit['evaluation']['summary']['functions'] = 2
        unit['unavailable'].append({'key': 'function:u0:f2', 'reason': 'missing capture'})
        self.assertEqual(compare(report, labels)['counts'], {'agreement': 2, 'unavailable': 2})

    def test_context_policy_and_unsupported_label_contracts_reject(self):
        for mutation in ('run', 'metric', 'policy', 'order'):
            report, labels = fixture()
            if mutation == 'run':
                report['run_directory'] = '/saved/other'
            elif mutation == 'metric':
                report['compiler_advisory']['metrics'] = 'other'
            elif mutation == 'policy':
                report['compiler_advisory']['effective_config']['fn_params']['max_params'] = 9
            else:
                labels['caller_relationship']['expected']['signature_steps'].reverse()
            with self.assertRaises(ValueError):
                compare(report, labels)


if __name__ == '__main__':
    unittest.main()
