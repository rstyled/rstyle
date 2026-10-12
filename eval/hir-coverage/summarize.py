"""Aggregate frozen eligibility observations; never load detector predictions."""
import argparse
import collections
import hashlib
import json
from pathlib import Path


def identity(function):
    return (function['package'], function['version'], function['path'], function['start'], function['end'], function['sha256'])


def summarize(stages):
    baseline = {identity(f): f for f in stages[0]['functions']}
    previous = set()
    slices = []
    for stage in stages:
        current = {identity(f): f for f in stage['functions']}
        if len(current) != len(stage['functions']) or current.keys() != baseline.keys():
            raise ValueError('Frozen source identities changed or duplicated')
        admitted = {key for key, f in current.items() if f['outcome'] == 'admitted'}
        slices.append(dict(stage=stage['stage'], counts=stage['counts'], newly_admitted=[dict(current[key], build=None) for key in sorted(admitted - previous)], lost_admissions=[list(key) for key in sorted(previous - admitted)]))
        previous = admitted
    final = stages[-1]['functions']
    reasons = collections.Counter(tuple(sorted(set(f['reasons']))) for f in final if f['outcome'] != 'admitted')
    syntax = collections.Counter(tuple(sorted(set(b['capability'] for b in f['source_blockers']))) for f in final if f['outcome'] != 'admitted')
    source_only = collections.defaultdict(list)
    for f in stages[0]['functions']:
        blockers = sorted(set(b['capability'] for b in f['source_blockers']))
        if blockers:
            source_only[' + '.join(blockers)].append(list(identity(f)))
    return dict(slices=slices, remaining_reason_combinations=[dict(reasons=list(r), functions=n) for r, n in sorted(reasons.items())], remaining_source_syntax_combinations=[dict(capabilities=list(c), functions=n) for c, n in sorted(syntax.items())], hypothetical_source_syntax_bundles=dict(sorted(source_only.items())), claim='Source-syntax bundles are only hypothetical candidates; signatures, resolution, conditional evidence and explicitly excluded forms can independently block them. Only newly_admitted records are measured gains. No quality predictions or calibration.')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('results', nargs='+', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = summarize([json.loads(path.read_text()) for path in args.results])
    result['result_sha256'] = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in args.results}
    args.output.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
