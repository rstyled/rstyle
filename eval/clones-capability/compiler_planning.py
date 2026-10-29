#!/usr/bin/env python3
"""Compare completed core parameter plans with frozen source relationships."""
import argparse
from collections import Counter
import json
from pathlib import Path

from compiler_quality import check_rule, planned_owners, sample_owner, units, validate_planning
from project_coverage import boolean, integer, load, require, unique


def completed_plan(group):
    plan = group['evaluation'].get('plan')
    if plan is None:
        require(not group['step_owners'] and not group['bundle_owners'],
                'owner rosters without a parameter plan')
        return {'steps': [], 'bundles': []}
    return plan


def indexed_plans(report):
    known = {row['key']: row['name'] for unit in report['compiler_advisory']['units']
             for row in unit['observations']}
    groups = report['compiler_advisory']['planning']['groups']
    for group in groups:
        plan = completed_plan(group)
        require(len(group['step_owners']) == len(plan['steps']), 'incomplete step-owner roster')
        require(len(set(group['step_owners'])) == len(group['step_owners']), 'duplicate step owner')
        for key, step in zip(group['step_owners'], plan['steps']):
            require(key in known and known[key] == step['name'], 'step owner differs from observation')
        require(len(group['bundle_owners']) == len(plan.get('bundles', [])), 'incomplete bundle-owner roster')
        for owners, bundle in zip(group['bundle_owners'], plan.get('bundles', [])):
            require(len(set(owners)) == len(owners), 'duplicate bundle owner')
            require(all(key in known for key in owners), 'unknown bundle owner')
            require(sorted({known[key] for key in owners}) == sorted(bundle['functions']),
                    'bundle owner differs from observation')
    return groups


def relationship(groups, labels, keys, reasons):
    relation = labels['caller_relationship']
    caller, callee = relation['caller'], relation['callee']
    if any(reasons.values()):
        return [{'check': 'caller_relationship', 'result': 'unavailable',
                 'reason': '; '.join(sorted(set(filter(None, reasons.values()))))}]
    caller_key, callee_key = keys[caller], keys[callee]
    matched = [group for group in groups
               if caller_key in group['members'] and callee_key in group['members']]
    checks = [('shared_edit_group', len(matched) == 1)]
    if len(matched) != 1:
        return [{'check': check, 'result': 'agreement' if ok else 'disagreement'}
                for check, ok in checks]
    group = matched[0]
    plan = completed_plan(group)
    steps = dict(zip(group['step_owners'], plan['steps']))
    indices = {key: index for index, key in enumerate(group['step_owners'])}
    expected = relation['expected']
    checks.append(('callee_before_caller',
                   all(key in steps and steps[key]['signature'] for key in (callee_key, caller_key))
                   and indices[callee_key] < indices[caller_key]))
    checks.append(('caller_pass_through', sorted(steps.get(caller_key, {}).get('pass_through', []))
                   == sorted(expected['caller_pass_through'])))
    wanted = {(param['name'], param['type']) for param in expected['bundle_contains']}
    checks.append(('shared_parameter_bundle', any(
        {caller_key, callee_key}.issubset(owners)
        and wanted.issubset({(param['name'], param['type']) for param in bundle['params']})
        for owners, bundle in zip(group['bundle_owners'], plan.get('bundles', [])))))
    for sample in labels['samples']:
        if not sample['expected']['fn_params_finding']:
            checks.append(('clean_control_signature:' + sample['function'], not any(
                key == keys[sample['function']] and step['signature']
                for other in groups for key, step in
                zip(other['step_owners'], completed_plan(other)['steps']))))
    return [{'check': check, 'result': 'agreement' if ok else 'disagreement'}
            for check, ok in checks]


def compare(report, labels):
    require(labels['schema'] == 'rstyle.compiler-parameter-plan-cases/1', 'wrong labels schema')
    require(Path(report['run_directory']).name == labels['run'], 'different saved run')
    require(report['toolchain'] == labels['toolchain'], 'different toolchain')
    advisory = report['compiler_advisory']
    require(advisory['metrics'] == labels['metrics'] == 'rstyle.compiler-style-metrics/6',
            'different compiler metrics')
    require(advisory['planning']['schema'] == labels['planning_schema']
            == 'rstyle.compiler-planning/1', 'different planning schema')
    for name, value in labels['policy'].items():
        require(advisory['effective_config'][name] == value, 'different policy: ' + name)
    owners = planned_owners(advisory['planning'])
    validate_planning(report, owners, ('fn_params',))
    groups = indexed_plans(report)
    selected = [(unit, observed) for unit, observed in units(report)
                if observed['target'].get('name') == labels['target_name']]
    keys, reasons, rows = {}, {}, []
    samples = unique(labels['samples'], 'function', 'source labels')
    relation = labels['caller_relationship']
    caller, callee = relation['caller'], relation['callee']
    require(caller != callee and caller in samples and callee in samples, 'invalid caller relation')
    require(relation['expected']['signature_steps'] == [callee, caller]
            and relation['expected']['relative_callee_before_caller'] is True,
            'unsupported signature ordering label')
    for name, label in samples.items():
        sample = {**labels, **label}
        unit, key, reason = sample_owner(report, sample, selected)
        keys[name], reasons[name] = key, reason
        actual = integer(label['expected']['params'], 'independent parameter count')
        limit = integer(labels['policy']['fn_params']['max_params'], 'independent limit')
        require(boolean(label['expected']['fn_params_finding'], 'finding label') == (actual > limit),
                'finding label contradicts source count')
        if reason:
            row = {'result': 'unavailable', 'reason': reason, 'observed': None}
        else:
            row = check_rule(unit, key, owners.get(key), sample, 'fn_params', actual, limit)
        rows.append({'function': name, 'key': key, **row})
    require(rows, 'empty source sample')
    checks = relationship(groups, labels, keys, reasons)
    return {'schema': 'rstyle.compiler-parameter-plan-results/1', 'run': labels['run'],
            'metrics': advisory['metrics'], 'scope_complete': advisory['complete'],
            'planning_input_complete': advisory['planning']['input_complete'],
            'counts': dict(sorted(Counter(row['result'] for row in rows + checks).items())),
            'parameters': rows, 'relationship': checks,
            'limitations': ['Only the frozen source owners and relationship are checked.',
                            'Clean numeric scores are not synthesized from absent diagnostics.',
                            'Plans are advisory over captured callers; no whole-program or rewrite-safety claim.']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('report', 'labels', 'output'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    result = compare(load(args.report), load(args.labels))
    with Path(args.output).open('x') as stream:
        json.dump(result, stream, indent=2)
        stream.write('\n')
    if result['counts'].get('unavailable') or result['counts'].get('disagreement'):
        parser.exit(2)


if __name__ == '__main__':
    main()
