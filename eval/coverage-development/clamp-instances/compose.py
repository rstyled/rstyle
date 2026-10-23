#!/usr/bin/env python3
"""Capture unchanged registry clamp owners and strictly replay contextual callers."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

import oracle


def expectations(report):
    for flag in ('capture_complete', 'assessment_complete',
                 'pair_enumeration_complete', 'pair_evaluation_complete'):
        assert report[flag], (flag, report.get('handoff_failures'))
    callers = {row['name']: row for row in report['functions']
               if row['unit_evidence']['report_unit'] == 0}
    assert len(callers) == 13
    for scalar in ('i32', 'u32'):
        for operation in ('clamp', 'clamp_min', 'clamp_max'):
            name = operation + '_' + scalar
            left, right = callers[name], callers[name + '_peer']
            assert left['outcome'] == right['outcome'] == 'admitted', (left, right)
            pair = next(pair for pair in report['matching']['pairs']
                        if {pair['first'], pair['second']} == {left['key'], right['key']})
            assert pair['decision'] == 'Accepted', pair
    assert callers['outside']['outcome'] == 'excluded'
    return [(row['key'], row['outcome'], row['reasons']) for row in report['functions']]


def run(command, environment, prefix):
    started = time.monotonic()
    with prefix.with_suffix('.json').open('w') as output, prefix.with_suffix('.log').open('w') as log:
        completed = subprocess.run(command, env=environment, stdout=output, stderr=log, timeout=180)
    assert completed.returncode == 0, (completed.returncode, str(prefix))
    return json.loads(prefix.with_suffix('.json').read_text()), time.monotonic() - started


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--cli', type=Path, default=Path('target/debug/cargo-rstyle'))
    parser.add_argument('--driver', type=Path, default=Path('target/debug/rstyle-driver'))
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / 'selection.json').read_text())
    original = Path(selection['package']['source_root']).resolve()
    oracle.verify_sources(selection, original)
    args.output.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, RSTYLE_DRIVER=str(args.driver.resolve()),
                       RUSTUP_TOOLCHAIN=selection['build']['toolchain'])
    for key in list(environment):
        if key.startswith(('CARGO_PROFILE_', 'CARGO_TARGET_')) or key in {
            'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
            'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET', 'RUSTC', 'RUSTDOC',
        }:
            environment.pop(key, None)
    results = []
    for context in selection['build']['contexts']:
        work = args.output / context['name']
        (work / 'src').mkdir(parents=True, exist_ok=True)
        (work / 'src/lib.rs').write_text((directory / 'callers.rs').read_text())
        (work / 'Cargo.toml').write_text(oracle.manifest(context))
        features = oracle.resolved_original('cargo', work, environment, original)
        assert sorted(features) == sorted(context['effective_features'])
        environment['CARGO_TARGET_DIR'] = str((work / 'target').resolve())
        base = [str(args.cli.resolve()), 'clones', '--extraction-profile', 'hir-data-v3',
                '--format', 'json', '--report-detail', 'compact']
        capture, capture_seconds = run(base + ['--manifest-path', str(work / 'Cargo.toml'),
                                      '--capture-dependency', 'num-traits@0.2.19', '--offline'],
                                      environment, work / 'capture')
        expected = expectations(capture)
        replay, replay_seconds = run(base + ['--replay-run', capture['run_directory']],
                                    environment, work / 'replay')
        assert replay['verification_policy'] == 'strict-replay'
        assert expectations(replay) == expected
        result = {'context': context['name'], 'caller_admissions': 12,
                  'negative_exclusions': 1, 'pairs': len(replay['matching']['pairs']),
                  'original_definitions': len(replay['functions']) - 13,
                  'capture_seconds': round(capture_seconds, 3),
                  'replay_seconds': round(replay_seconds, 3)}
        results.append(result)
        print(json.dumps(result), flush=True)
        (args.output / 'summary.json').write_text(json.dumps(results, indent=2) + '\n')


if __name__ == '__main__':
    main()
