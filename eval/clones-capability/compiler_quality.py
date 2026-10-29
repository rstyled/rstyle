#!/usr/bin/env python3
"""Review a labeled compiler sample using completed core plans, without reanalysis."""
import argparse
from collections import Counter
import json
from pathlib import Path

from compare import legacy_assessment
from density_compare import KINDS, label_error, policy_error
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
        for rule in ('fn_params', 'mutated_targets', 'side_effect_density'):
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
    if 'source' in sample and sources[0].get('source') != sample['source']:
        return unit, None, 'independent source label differs from captured source'
    return unit, owner['key'], None


def check_rule(unit, key, function, sample, rule, actual, limit, expected=None):
    wanted = expected or {'outcome': 'finding' if actual > limit else 'evaluated_no_finding',
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
    if observed is not None and observed['outcome'] == 'finding':
        integer(observed.get('actual'), 'observed actual')
        integer(observed.get('limit'), 'observed limit')
    agrees = observed is not None and observed['outcome'] == wanted['outcome']
    if agrees and wanted['outcome'] == 'finding':
        agrees = all(observed.get(field) == wanted[field] for field in wanted)
    return {'key': key, 'rule': rule, 'expected': wanted, 'observed': observed,
            'result': 'unavailable' if reason else 'agreement' if agrees else 'disagreement',
            'reason': reason}


def density_details(unit, function):
    diagnostic = next(row for row in unit['evaluation']['diagnostics']
                      if row['rule_id'] == 'side_effect_density' and row['path'] == function['path']
                      and row['line'] == function['line'])
    return diagnostic.get('details')


def density_events(details, *, direct_only):
    if not isinstance(details, list):
        return None
    counts = Counter()
    for detail in details:
        if not isinstance(detail, dict):
            return None
        label = detail.get('label')
        if not isinstance(label, str) or ': ' not in label:
            return None
        if direct_only and ' (via ' in label:
            return None
        kind = label.split(': ', 1)[0]
        if kind not in KINDS:
            return None
        count = integer(detail.get('count'), 'effect detail count')
        require(count > 0, 'effect detail count must be positive')
        counts[kind] += count
    return dict(counts)


def detail_roster(details):
    return sorted(({'label': detail['label'], 'count': detail['count']} for detail in details),
                  key=lambda detail: (detail['label'], detail['count']))


def reviewed_details(expected, policy):
    if policy['transitive_depth'] == 0:
        return None
    details = expected.get('details')
    events = density_events(details, direct_only=False)
    weighted = {kind: count for kind, count in expected['events'].items() if policy['weights'][kind] > 0}
    require(events is not None and events == weighted, 'reviewed weighted detail roster differs from events')
    require(len({detail['label'] for detail in details}) == len(details), 'duplicate reviewed detail label')
    return detail_roster(details)


def propagation_reason(report, sample, selected, depth):
    if depth == 0:
        return None
    require(sample.get('propagation_reviewed') is True, 'propagation source inventory not reviewed')
    inputs = sample.get('propagation_inputs')
    require(isinstance(inputs, list), 'propagation input roster missing')
    identities = set()
    for entry in inputs:
        require(isinstance(entry, dict) and entry.get('included') is True
                and entry.get('unsuppressed') is True and isinstance(entry.get('source'), str)
                and entry['source'], 'propagation input source policy not reviewed')
        identity = tuple(entry[field] for field in ('package', 'target_kind', 'function', 'source_file'))
        require(identity not in identities, 'duplicate propagation input')
        identities.add(identity)
        _, _, reason = sample_owner(report, entry, selected)
        if reason:
            return 'sampled propagation input unavailable: ' + reason
    return None


def check_density(unit, key, function, sample, policy, reason):
    expected = sample.get('density')
    error = label_error({'expected': expected}, policy)
    require(error is None, error)
    require(expected['complete'] is True, 'density sample requires reviewed complete static events')
    wanted_details = reviewed_details(expected, policy)
    require(sample.get('included') is True and isinstance(sample.get('source'), str)
            and sample['source'], 'density source and inclusion preconditions missing')
    wanted = {field: expected[field] for field in ('outcome', 'actual')}
    wanted['limit'] = policy['max_density_milli']
    if reason:
        return {'key': key, 'rule': 'side_effect_density', 'expected': wanted,
                'observed': None, 'result': 'unavailable', 'reason': reason}
    row = check_rule(unit, key, function, sample, 'side_effect_density',
                     expected['actual'], policy['max_density_milli'], wanted)
    observation = next(entry for entry in unit['observations'] if entry['key'] == key)
    statements = integer(observation.get('statements'), 'observed statements')
    row['statements'] = {'expected': expected['statements'], 'observed': statements}
    if row['result'] != 'unavailable' and statements != expected['statements']:
        row['result'] = 'disagreement'
    if row['observed'] and row['observed']['outcome'] == 'finding':
        details = density_details(unit, function)
        events = density_events(details, direct_only=policy['transitive_depth'] == 0)
        if not events:
            row.update(result='unavailable', reason='weighted event details unavailable')
        else:
            weighted = {kind: count for kind, count in expected['events'].items() if policy['weights'][kind] > 0}
            row['events'] = {'expected': weighted, 'observed': events}
            if events != weighted:
                row['result'] = 'disagreement'
            if wanted_details is not None:
                row['details'] = {'expected': wanted_details, 'observed': detail_roster(details)}
                if row['details']['expected'] != row['details']['observed']:
                    row['result'] = 'disagreement'
    return row


def compare(report, labels):
    require(labels['schema'] == 'rstyle.compiler-quality-cases/1', 'wrong labels schema')
    require(Path(report['run_directory']).name == labels['run'], 'different saved run')
    require(report['toolchain'] == labels['toolchain'], 'different toolchain')
    advisory = report['compiler_advisory']
    boolean(advisory.get('complete'), 'advisory completion')
    require(advisory['schema'] == 'rstyle.compiler-advisory/1', 'wrong advisory schema')
    require(advisory['metrics'] == labels['metrics'], 'different compiler metric')
    rules = set(labels['limits'])
    require(rules and rules <= {'fn_params', 'mutated_targets', 'side_effect_density'}, 'unknown rule limits')
    for rule, limit in labels['limits'].items():
        integer(limit, rule + ' limit')
    require(isinstance(labels['samples'], list) and labels['samples'], 'empty sample')
    config = advisory['effective_config']
    require(config['exclude'] == ['target/**'], 'different sample exclusions')
    if 'fn_params' in rules:
        require(config['fn_params']['max_params'] == labels['limits']['fn_params']
                and config['fn_params']['count_self'] is False, 'different parameter policy')
    if 'mutated_targets' in rules:
        require(config['mutation_budget']['max_mutated_targets'] == labels['limits']['mutated_targets']
                and config['mutation_budget']['granularity'] == 'root', 'different mutation policy')
    if 'side_effect_density' in rules:
        density_policy = labels.get('density_policy')
        error = policy_error(density_policy, depths=(0, 1, 2))
        require(error is None, error)
        observed_policy = {**config['side_effects'], 'exclude': config['exclude']}
        error = policy_error(observed_policy, depths=(0, 1, 2))
        require(error is None, error)
        require(density_policy['max_density_milli'] == labels['limits']['side_effect_density']
                and density_policy == observed_policy,
                'different complete density policy')
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
        measurements = {}
        if 'fn_params' in rules:
            measurements['fn_params'] = sample['parameters']
        if 'mutated_targets' in rules:
            roots = sample['mutated_roots']
            require(isinstance(roots, list) and all(isinstance(root, str) and root for root in roots)
                    and len(set(roots)) == len(roots), 'invalid independent root labels')
            measurements['mutated_targets'] = len(roots)
        unit, key, reason = sample_owner(report, sample, selected)
        if 'side_effect_density' in rules:
            propagation = propagation_reason(report, sample, selected, density_policy['transitive_depth'])
            reason = reason or propagation
            row = check_density(unit, key, owners.get(key), sample, density_policy, reason)
            rows.append({**{field: sample[field] for field in ('package', 'function', 'source_file')}, **row})
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
