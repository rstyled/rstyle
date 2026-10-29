#!/usr/bin/env python3
"""Review completed compiler proposals against independent source-role labels."""
import argparse
from collections import Counter
import json
from pathlib import Path

from compiler_quality import sample_owner, units
from project_coverage import boolean, integer, load, require, relative_path, unique

KINDS = {'trait_method', 'capability_set', 'enum_dispatch', 'generic_fn', 'fn_param', 'parameterize'}
JUDGMENTS = {'useful', 'needs_revision', 'not_useful'}
ROLES = ('sites', 'definitions')


def policy_checked(policy):
    fields = {'threshold_milli', 'min_coverage_milli', 'pair_coverage_milli',
              'normalize', 'single_call_guard', 'max_holes'}
    require(isinstance(policy, dict) and set(policy) == fields, 'complete proposal policy missing')
    for field in ('threshold_milli', 'min_coverage_milli', 'max_holes'):
        integer(policy[field], field)
    if policy['pair_coverage_milli'] is not None:
        integer(policy['pair_coverage_milli'], 'pair coverage')
    boolean(policy['single_call_guard'], 'single call guard')
    rules = policy['normalize']
    require(isinstance(rules, dict) and set(rules) == {'ctor', 'pred', 'case', 'exit', 'counter', 'cmp', 'propagate'},
            'complete normalization policy missing')
    for value in rules.values():
        boolean(value, 'normalization policy')
    return policy


def role_key(candidate):
    require(candidate['kind'] in KINDS, 'unknown proposal family')
    roles = []
    for role in ROLES:
        values = [entry['id'] for entry in candidate[role]]
        require(all(isinstance(value, str) and value for value in values)
                and len(values) == len(set(values)), 'duplicate or malformed proposal member')
        roles.append(tuple(sorted(values)))
    require(roles[0], 'empty proposal sites')
    return candidate['kind'], *roles


def captured(report, labels):
    selected = units(report)
    inventory, proposals, coverage = [], {}, {}
    for unit, _ in selected:
        patterns = unit.get('patterns')
        if patterns is None:
            continue
        require(patterns['schema'] == 'rstyle.compiler-patterns/1', 'wrong proposal schema')
        require(policy_checked(patterns['policy']) == labels['policy'], 'different proposal policy')
        boolean(patterns['input_complete'], 'proposal input completion')
        candidates = patterns['evaluation']['candidates']
        require(isinstance(candidates, list), 'proposal roster missing')
        visible = patterns.get('visible_candidates')
        if visible is not None:
            require(isinstance(visible, list) and len(set(visible)) == len(visible), 'invalid visibility roster')
            for index in visible:
                require(integer(index, 'visible candidate') < len(candidates), 'invalid visible candidate')
        missing = unique(patterns['unavailable_graphs'], 'key', 'missing proposal graphs')
        count = integer(patterns['policy_selected_functions'], 'selected proposal owners')
        graphs = integer(patterns['observed_graphs'], 'observed proposal graphs')
        known = unique(unit['observations'], 'key', 'known proposal owners')
        known_roster = (count == len(known) and graphs + len(missing) == count
                        and set(missing) <= set(known))
        coverage[unit['unit']] = (patterns, missing, known_roster)
        for index, candidate in enumerate(candidates):
            key = unit['unit'], role_key(candidate)
            require(key not in proposals, 'duplicate proposal role identity')
            row = {'unit': unit['unit'], 'index': index, 'id': candidate['id'], 'kind': candidate['kind'],
                   'sampled_by': [], 'visible': visible is None or index in visible}
            for field in ('score_milli', 'breakdown', 'counter_evidence', *ROLES):
                if field in candidate:
                    row[field] = candidate[field]
            inventory.append(row)
            proposals[key] = candidate, row
    return selected, inventory, proposals, coverage


def owners(report, sample, selected, coverage):
    unit_id, identities = None, {}
    for role in ROLES:
        require(isinstance(sample[role], list), 'missing source role labels')
        for owner in sample[role]:
            require(owner.get('included') is True and owner.get('unsuppressed') is True
                    and isinstance(owner.get('source'), str) and owner['source'], 'unreviewed source owner')
            require(isinstance(owner.get('compiler_id'), str) and owner['compiler_id'], 'missing compiler owner identity')
            unit, key, reason = sample_owner(report, owner, selected)
            if reason:
                return None, identities, reason
            if unit_id is not None and unit['unit'] != unit_id:
                return None, identities, 'proposal members span compilation units'
            unit_id = unit['unit']
            state = coverage.get(unit_id)
            if state is None:
                return unit_id, identities, 'compiler proposals not captured'
            _, missing, known_roster = state
            if key in missing:
                return unit_id, identities, missing[key]['reason']
            if not known_roster:
                return unit_id, identities, 'selected proposal owner roster unavailable'
            previous = identities.get(owner['compiler_id'])
            require(previous is None or previous == (owner, unit), 'conflicting source owner labels')
            identities[owner['compiler_id']] = owner, unit
    return unit_id, identities, None


def inspect_sample(report, sample, selected, proposals, coverage):
    require(sample['judgment'] in JUDGMENTS and isinstance(sample.get('rationale'), str)
            and sample['rationale'], 'independent usefulness review missing')
    require(all(isinstance(sample[role], list) for role in ROLES), 'missing source role labels')
    expected = {'kind': sample['kind'], **{role: [{'id': owner['compiler_id']} for owner in sample[role]] for role in ROLES}}
    identity = role_key(expected)
    unit, source_owners, reason = owners(report, sample, selected, coverage)
    row = {'id': sample['id'], 'result': 'unavailable', 'reason': reason}
    if reason:
        return row
    match = proposals.get((unit, identity))
    if match is None:
        complete = coverage[unit][0]['input_complete']
        row.update(result='not_observed' if complete else 'unavailable',
                   reason='labeled proposal not observed' if complete else 'proposal absent over incomplete input')
        return row
    candidate, inventory = match
    for role in ROLES:
        for site in candidate[role]:
            owner, context = source_owners[site['id']]
            if (site['name'] != owner['function']
                    or relative_path(site['path'], context['source_root']) != owner['source_file']):
                row['reason'] = 'proposal member differs from labeled source owner'
                return row
    inventory['sampled_by'].append(sample['id'])
    row.update(result='structural_agreement', reason=None, unit=unit, index=inventory['index'],
               judgment=sample['judgment'], rationale=sample['rationale'])
    return row


def compare(report, labels):
    require(labels['schema'] == 'rstyle.compiler-proposal-quality-cases/1', 'wrong proposal labels schema')
    require(Path(report['run_directory']).name == labels['run'], 'different saved run')
    require(report['toolchain'] == labels['toolchain'], 'different toolchain')
    advisory = report['compiler_advisory']
    require(advisory['schema'] == 'rstyle.compiler-advisory/1', 'wrong advisory schema')
    require(advisory['metrics'] == labels['metrics'], 'different compiler metric')
    boolean(advisory['complete'], 'advisory completion')
    policy_checked(labels['policy'])
    require(isinstance(labels['samples'], list) and labels['samples'], 'empty proposal sample')
    unique(labels['samples'], 'id', 'proposal source labels')
    selected, inventory, proposals, coverage = captured(report, labels)
    rows = [inspect_sample(report, sample, selected, proposals, coverage) for sample in labels['samples']]
    return {'schema': 'rstyle.compiler-proposal-quality-observation/1', 'run': labels['run'],
            'metrics': advisory['metrics'], 'policy': labels['policy'],
            'scope_complete': advisory['complete'],
            'input_complete': {str(unit): state[0]['input_complete'] for unit, state in coverage.items()},
            'sample_complete': all(row['result'] != 'unavailable' for row in rows),
            'counts': dict(sorted(Counter(row['result'] for row in rows).items())),
            'judgments': dict(sorted(Counter(row['judgment'] for row in rows if 'judgment' in row).items())),
            'samples': rows, 'inventory': inventory,
            'limitation': 'Source-role agreement is separate from usefulness. Unreviewed raw candidates remain in inventory; incomplete input cannot establish absent proposals or whole-project quality.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ('report', 'labels', 'output'):
        parser.add_argument('--' + field, required=True)
    args = parser.parse_args()
    try:
        result = compare(load(args.report), load(args.labels))
        with Path(args.output).open('x') as stream:
            json.dump(result, stream, indent=2)
            stream.write('\n')
        if result['counts'].get('unavailable', 0):
            parser.exit(2)
        if result['judgments'].get('not_useful', 0) or result['judgments'].get('needs_revision', 0):
            parser.exit(1)
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.exit(2, 'compiler proposal quality: ' + str(error) + '\n')


if __name__ == '__main__':
    main()
