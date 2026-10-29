#!/usr/bin/env python3
"""Compare completed core complexity diagnostics with independently frozen source labels."""
import argparse
from collections import Counter
import json
from pathlib import Path

from compiler_quality import check_rule, planned_owners, sample_owner, units, validate_planning
from project_coverage import boolean, integer, load, require, unique

RULES = ('cyclomatic_complexity', 'cognitive_complexity')


def ranked_scores(unit):
    ranking = unit.get('complexity_ranking')
    if ranking is None:
        return None
    require(isinstance(ranking, dict)
            and ranking.get('schema') == 'rstyle.compiler-complexity-ranking/1',
            'wrong complexity ranking schema')
    counts = {name: integer(ranking.get(name), name) for name in (
        'requested_limit', 'known_functions', 'measured_functions',
        'unavailable_measurements', 'unavailable_owners')}
    require(counts['requested_limit'] > 0, 'invalid ranking limit')
    require(counts['known_functions'] == counts['measured_functions'] + counts['unavailable_measurements'],
            'inconsistent ranking measurement counts')
    require(counts['unavailable_owners'] == len(unit['unavailable']), 'inconsistent missing owner count')
    require(boolean(ranking.get('input_complete'), 'ranking input completeness')
            == (counts['unavailable_measurements'] == counts['unavailable_owners'] == 0),
            'inconsistent ranking input completeness')
    summary = unit['evaluation']['summary']
    for field, counter in [('functions', 'known_functions'), ('complexity_measured', 'measured_functions'),
                           ('complexity_unavailable', 'unavailable_measurements')]:
        if field in summary:
            require(integer(summary[field], field) == counts[counter], 'ranking differs from core summary')
    owners, functions = ranking.get('owners'), ranking.get('functions')
    require(isinstance(owners, list) and isinstance(functions, list)
            and len(owners) == len(functions) == min(counts['known_functions'], counts['requested_limit']),
            'incomplete ranking owner roster')
    known = unique(unit['observations'], 'key', 'known observations')
    require(counts['known_functions'] <= len(known), 'ranked count exceeds observed owners')
    result, ids = {}, set()
    for key, function in zip(owners, functions):
        require(isinstance(key, str) and key in known and key not in result,
                'missing, duplicate or foreign ranked owner')
        require(isinstance(function, dict) and function.get('name') == known[key]['name'],
                'ranked owner name mismatch')
        identity = function.get('id')
        require(isinstance(identity, str) and bool(identity) and identity not in ids,
                'missing or duplicate ranked identity')
        require(isinstance(function.get('path'), str) and bool(function['path']), 'missing ranked source path')
        require(integer(function.get('line'), 'ranked line') > 0
                and integer(function.get('column'), 'ranked column') > 0, 'invalid ranked source location')
        require('complexity' in function, 'missing ranked score availability')
        if function['complexity'] is not None:
            score_values(function['complexity'])
        ids.add(identity)
        result[key] = function
    measured = sum(row['complexity'] is not None for row in result.values())
    require(measured <= counts['measured_functions']
            and len(result) - measured <= counts['unavailable_measurements'], 'inconsistent displayed scores')
    return result


def score_values(score):
    require(isinstance(score, dict), 'invalid complexity score')
    return {name: integer(score.get(name), name) for name in ('policy_version', 'cyclomatic', 'cognitive')}


def check_score(sample, key, scores, reason):
    expected = score_values(sample['expected'])
    observed = None
    if reason is None:
        row = scores.get(key) if scores is not None else None
        if row is None:
            reason = 'Completed ranked score is unavailable or outside the requested display limit'
        elif row['complexity'] is None:
            reason = 'Compiler complexity measurement is unavailable'
        else:
            require(row['path'] == sample['source_file'], 'ranked source path differs')
            observed = score_values(row['complexity'])
    return {'key': key, 'expected': expected, 'observed': observed,
            'result': 'unavailable' if reason else 'agreement' if expected == observed else 'disagreement',
            'reason': reason}


def compare(report, labels, require_scores=False):
    require(labels['schema'] == 'rstyle.compiler-complexity-cases/1', 'wrong labels schema')
    require(Path(report['run_directory']).name == labels['run'], 'different saved run')
    require(report['toolchain'] == labels['toolchain'], 'different toolchain')
    advisory = report['compiler_advisory']
    require(advisory['metrics'] == labels['metrics'] == 'rstyle.compiler-style-metrics/6',
            'complexity metric unavailable')
    require(labels['complexity_policy_version'] == 1, 'different complexity policy')
    require(advisory['effective_config']['complexity'] == labels['limits'], 'different limits')
    owners = planned_owners(advisory['planning'])
    validate_planning(report, owners, RULES)
    selected = units(report)
    scores = {unit['unit']: ranked_scores(unit) for unit, _ in selected} if require_scores else {}
    rows, score_rows = [], []
    for sample in labels['samples']:
        unit, key, reason = sample_owner(report, sample, selected)
        require(sample['expected']['available'] is True, 'missing independent availability label')
        require(sample['expected']['policy_version'] == 1, 'different sample complexity policy')
        if reason is None:
            missing = unique(unit['unavailable_complexity'], 'key', 'unavailable complexity')
            if key in missing:
                reason = missing[key].get('reason') or 'Compiler complexity is unavailable'
        if require_scores:
            score_rows.append({'function': sample['function'], 'source_file': sample['source_file'],
                               'test': sample['test'],
                               **check_score(sample, key, scores.get(unit['unit']) if unit else None, reason)})
        for rule in RULES:
            stem = rule.removesuffix('_complexity')
            actual = integer(sample['expected'][stem], 'independent score')
            limit = integer(labels['limits']['max_' + stem], 'independent limit')
            if reason:
                row = {'key': key, 'rule': rule, 'result': 'unavailable',
                       'reason': reason, 'observed': None}
            else:
                row = check_rule(unit, key, owners.get(key), sample, rule, actual, limit)
            rows.append({'function': sample['function'], 'source_file': sample['source_file'],
                         'test': sample['test'], **row})
    require(rows, 'empty sample')
    result = {'schema': 'rstyle.compiler-complexity-results/1', 'run': labels['run'],
            'metrics': advisory['metrics'], 'scope_complete': advisory['complete'],
            'coverage': [{'unit': unit['unit'], 'package': observed['target']['package_context']['name'],
                          'target_kind': observed['target']['kind'], 'test': observed['test'],
                          'captured_functions': unit['expected_functions'],
                          'evaluation': unit['evaluation']['summary'],
                          'unavailable_owners': len(unit['unavailable']),
                          'unavailable_complexity': len(unit['unavailable_complexity']),
                          'unresolved_callees': len(unit['unresolved_callees'])}
                         for unit, observed in selected],
            'counts': dict(sorted(Counter(row['result'] for row in rows).items())),
            'comparisons': rows,
            'limitation': 'Sample outcomes only; clean numeric scores are not in diagnostics and are never synthesized.'}
    if require_scores:
        result.update(score_counts=dict(sorted(Counter(row['result'] for row in score_rows).items())),
                      score_comparisons=score_rows,
                      limitation='Sample outcomes and explicitly requested completed ranked scores only; absent or truncated measurements remain unavailable. No rescoring or whole-project quality claim.')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('report', 'labels', 'output'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--require-scores', action='store_true',
                        help='Also require exact completed scores, including clean functions, from compiler complexity rankings')
    args = parser.parse_args()
    result = compare(load(args.report), load(args.labels), args.require_scores)
    with Path(args.output).open('x') as stream:
        json.dump(result, stream, indent=2)
        stream.write('\n')
    failed = ('unavailable', 'disagreement')
    if any(result[field].get(state) for field in ('counts', 'score_counts')
           for state in failed if field in result):
        parser.exit(2)


if __name__ == '__main__':
    main()
