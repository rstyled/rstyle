#!/usr/bin/env python3
"""Compare completed core complexity diagnostics with independently frozen source labels."""
import argparse
from collections import Counter
import json
from pathlib import Path

from compiler_quality import check_rule, planned_owners, sample_owner, units, validate_planning
from project_coverage import integer, load, require, unique

RULES = ('cyclomatic_complexity', 'cognitive_complexity')


def compare(report, labels):
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
    rows = []
    for sample in labels['samples']:
        unit, key, reason = sample_owner(report, sample, selected)
        require(sample['expected']['available'] is True, 'missing independent availability label')
        require(sample['expected']['policy_version'] == 1, 'different sample complexity policy')
        if reason is None:
            missing = unique(unit['unavailable_complexity'], 'key', 'unavailable complexity')
            if key in missing:
                reason = missing[key].get('reason') or 'Compiler complexity is unavailable'
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
    return {'schema': 'rstyle.compiler-complexity-results/1', 'run': labels['run'],
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
