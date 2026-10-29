#!/usr/bin/env python3
"""Review a labeled compiler sample using completed core plans, without reanalysis."""
import argparse
from collections import Counter
import json
from pathlib import Path

from compare import legacy_assessment
from project_coverage import boolean, integer, load, require, relative_path, unique


def units(report):
    contexts = unique(report['unit_contexts'], 'id', 'unit contexts')
    result = []
    for unit in report['compiler_advisory']['units']:
        context = contexts[unit['unit']]
        observed = report['observed_units'][context['observed_unit']]
        result.append((unit, observed))
    return result


def planned_owners(planning):
    result = {}
    for group in planning['groups']:
        members, functions = group['members'], group['evaluation']['functions']
        require(len(members) == len(functions), 'incomplete planned owner roster')
        for key, function in zip(members, functions):
            require(key not in result, 'duplicate planned owner')
            result[key] = function
    return result


def validate_planning(report, owners):
    known = {row['key']: (unit['unit'], row['name'])
             for unit in report['compiler_advisory']['units'] for row in unit['observations']}
    for key, function in owners.items():
        require(key in known and known[key][1] == function['name'], 'planned owner differs from observation')
    for unit in report['compiler_advisory']['units']:
        diagnostics = unit['evaluation']['diagnostics']
        require(unit['evaluation']['summary']['violations'] == len(diagnostics), 'incomplete diagnostics')
        for rule in ('fn_params', 'mutated_targets'):
            planned = sum(rule in function['rules'] for key, function in owners.items()
                          if known[key][0] == unit['unit'])
            require(planned == sum(row['rule_id'] == rule for row in diagnostics),
                    'incomplete completed rule-owner roster')


def sample_owner(report, sample, selected):
    candidates = [(unit, observed) for unit, observed in selected
                  if observed['target']['package_context']['name'] == sample['package']
                  and observed['target']['kind'] == [sample['target_kind']]
                  and observed['test'] is False]
    if len(candidates) != 1:
        return None, None, 'missing or ambiguous check unit'
    unit, observed = candidates[0]
    owners = [row for row in report['functions']
              if row['unit_evidence']['report_unit'] == unit['unit']
              and row['name'] == sample['function']]
    if len(owners) != 1:
        return unit, None, 'missing or ambiguous function owner'
    owner = owners[0]
    known = unique(unit['observations'], 'key', 'known observations')
    missing = unique(unit['unavailable'], 'key', 'unavailable observations')
    require(not (set(known) & set(missing)), 'conflicting availability')
    if owner['key'] not in known:
        return unit, None, missing.get(owner['key'], {}).get('reason', 'observation unavailable')
    require(known[owner['key']]['name'] == sample['function'], 'observation owner mismatch')
    sources = [source for inventory in report['owned_source_inventory']
               if inventory['compilation_unit'] == observed['target']
               and inventory['test'] is False
               for source in inventory['source_functions']
               if source['function_span'] == owner['fn_span']
               and sample['function'] in source['active_functions']
               and relative_path(source['source_file'], unit['source_root']) == sample['source_file']]
    if len(sources) != 1:
        return unit, None, 'original source association unavailable or ambiguous'
    return unit, owner['key'], None


def check_rule(unit, key, function, sample, rule, actual, limit):
    wanted = {'outcome': 'finding' if actual > limit else 'evaluated_no_finding',
              'actual': actual, 'limit': limit}
    observed = {'outcome': 'evaluated_no_finding'}
    reason = None
    if function and rule in function['rules']:
        require(function['path'] == sample['source_file'], 'planned source path differs')
        metadata = {'complete': True, 'functions': [{
            'name': sample['function'], 'source_file': function['path'],
            'line': function['line'], 'excluded': False}]}
        observed, reason = legacy_assessment(unit['evaluation'], metadata, sample['function'], rule)
        if observed is not None and rule == 'mutated_targets':
            diagnostic = next(row for row in unit['evaluation']['diagnostics']
                              if row['rule_id'] == rule and row['path'] == function['path']
                              and row['line'] == function['line'])
            if diagnostic.get('details') is None:
                observed, reason = None, 'mutation target details unavailable'
            else:
                wanted['roots'] = sorted(sample['mutated_roots'])
                observed['roots'] = sorted(row['label'] for row in diagnostic['details'])
    agrees = observed is not None and observed['outcome'] == wanted['outcome']
    if agrees and wanted['outcome'] == 'finding':
        agrees = all(observed.get(field) == wanted[field] for field in wanted)
    return {'key': key, 'rule': rule, 'expected': wanted, 'observed': observed,
            'result': 'unavailable' if reason else 'agreement' if agrees else 'disagreement',
            'reason': reason}


def compare(report, labels):
    require(labels['schema'] == 'rstyle.compiler-quality-cases/1', 'wrong labels schema')
    require(Path(report['run_directory']).name == labels['run'], 'different saved run')
    require(report['toolchain'] == labels['toolchain'], 'different toolchain')
    advisory = report['compiler_advisory']
    boolean(advisory.get('complete'), 'advisory completion')
    require(advisory['schema'] == 'rstyle.compiler-advisory/1', 'wrong advisory schema')
    require(advisory['metrics'] == labels['metrics'], 'different compiler metric')
    require(set(labels['limits']) == {'fn_params', 'mutated_targets'}, 'unknown rule limits')
    for rule, limit in labels['limits'].items():
        integer(limit, rule + ' limit')
    require(isinstance(labels['samples'], list) and labels['samples'], 'empty sample')
    config = advisory['effective_config']
    require(config['fn_params']['max_params'] == labels['limits']['fn_params']
            and config['fn_params']['count_self'] is False
            and config['mutation_budget']['max_mutated_targets'] == labels['limits']['mutated_targets']
            and config['mutation_budget']['granularity'] == 'root'
            and config['exclude'] == ['target/**'], 'different sample policy')
    planning = advisory['planning']
    boolean(planning.get('input_complete'), 'planning input completion')
    require(planning['schema'] == 'rstyle.compiler-planning/1', 'wrong planning schema')
    owners = planned_owners(planning)
    validate_planning(report, owners)
    selected = units(report)
    rows = []
    seen = set()
    for sample in labels['samples']:
        identity = (sample['package'], sample['target_kind'], sample['function'], sample['source_file'])
        require(identity not in seen, 'duplicate label identity')
        seen.add(identity)
        require(sample.get('unsuppressed') is True, 'sample suppression precondition not reviewed')
        roots = sample['mutated_roots']
        require(isinstance(roots, list) and all(isinstance(root, str) and root for root in roots)
                and len(set(roots)) == len(roots), 'invalid independent root labels')
        unit, key, reason = sample_owner(report, sample, selected)
        measurements = {'fn_params': sample['parameters'], 'mutated_targets': len(sample['mutated_roots'])}
        for rule, actual in measurements.items():
            require(type(actual) is int and actual >= 0, 'invalid source label')
            if reason:
                row = {'key': key, 'rule': rule, 'result': 'unavailable', 'reason': reason,
                       'expected': {'actual': actual, 'limit': labels['limits'][rule]}, 'observed': None}
            else:
                row = check_rule(unit, key, owners.get(key), sample, rule, actual, labels['limits'][rule])
            rows.append({**{field: sample[field] for field in ('package', 'function', 'source_file')}, **row})
    return {'schema': 'rstyle.compiler-quality-observation/1',
            'sample_complete': all(row['result'] != 'unavailable' for row in rows), 'run': labels['run'],
            'metrics': advisory['metrics'], 'effective_config': config,
            'scope_complete': advisory['complete'], 'planning_input_complete': planning['input_complete'],
            'counts': dict(sorted(Counter(row['result'] for row in rows).items())), 'comparisons': rows,
            'measurement_limit': 'No-finding rows compare outcomes only; clean numeric values are absent and are never synthesized.',
            'limitation': labels['limitations'],
            'preconditions': labels['preconditions']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', required=True)
    parser.add_argument('--labels', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    try:
        result = compare(load(args.report), load(args.labels))
        with Path(args.output).open('x') as stream:
            json.dump(result, stream, indent=2)
            stream.write('\n')
        if result['counts'].get('unavailable', 0):
            parser.exit(2)
        if result['counts'].get('disagreement', 0):
            parser.exit(1)
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.exit(2, 'compiler quality: ' + str(error) + '\n')


if __name__ == '__main__':
    main()
