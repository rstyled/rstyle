"""Measure prebuilt before/after tools, with no concurrent builds or tests.

Usage: python3 scripts/measure_library_resolution.py before|after BIN_ROOT OUTPUT
BIN_ROOT contains debug/ and release/ CLI, driver and unit_import_replay binaries
(the replay binary may be in examples/). Reuse OUTPUT across revisions for parity
checks. Warmup and diagnostic runs are excluded from ordinary medians.
"""
import json
import os
import pathlib
import re
import statistics
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
FLAGS = ['capture_complete', 'assessment_complete',
         'pair_enumeration_complete', 'pair_evaluation_complete']


def workloads():
    source = (REPO / 'crates/cargo-rstyle/tests/cli.rs').read_text()
    read = json.loads(re.search('const DATA_READ_CALL_FAMILY: &str = (".*?");', source).group(1))
    inherent = re.search('const DATA_INHERENT_FAMILY: &str = r#"(.*?)"#;', source, re.S).group(1)
    return [('minimal', 'r0', 'pub fn first(a:i32)->i32 {a}\npub fn second(a:i32)->i32 {a}\n', 2),
            ('read', 'hir-data-v2', read, 20), ('inherent', 'hir-data-v3', inherent, 21)]


def run(command, directory, output, tag, env):
    with (output / (tag + '.json')).open('w') as stdout, (output / (tag + '.err')).open('w') as stderr:
        subprocess.run(['/usr/bin/time', '-f', '%e %M', '-o', str(output / (tag + '.time')),
                        'timeout', '120', *map(str, command)], cwd=directory, env=env,
                       stdout=stdout, stderr=stderr, check=True)
    wall, rss = (output / (tag + '.time')).read_text().split()
    return json.loads((output / (tag + '.json')).read_text()), dict(wall=float(wall), rss=int(rss))


def signature(report, count):
    assert all(report[flag] is True for flag in FLAGS)
    assert len(report['functions']) == count
    assert all(function['outcome'] == 'admitted' for function in report['functions'])
    assert len(report['matching']['pairs']) == count * (count - 1) // 2
    names = {function['key']: function['facts']['name'] for function in report['functions']}
    result = sorted([sorted([names[pair['first']], names[pair['second']]]), pair['decision'],
                     pair.get('displayed_score'), [(stage['stage'], stage['outcome']) for stage in pair['stages']]]
                    for pair in report['matching']['pairs'])
    return json.loads(json.dumps(result))


def library_inventory(report):
    units = {}
    for function in report['functions']:
        options = function['facts']['build']['frontend_options']
        units[function['unit_evidence']['bundle']] = options.get('data_library_artifacts', {}).get('libraries', [])
    files = [file for libraries in units.values() for library in libraries for file in library.get('files', [])]
    # These are the retired explicit fingerprint reads, not rustc's own IO.
    return dict(fingerprint_bytes=sum(file['bytes'] for file in files), fingerprint_files=len(files))


def measure_workload(phase, profile, bins, output, workload):
    name, selection, source, count = workload
    directory = output / (profile + '-' + name)
    (directory / 'src').mkdir(parents=True, exist_ok=True)
    (directory / 'Cargo.toml').write_text("[package]\nname='profile_fixture'\nversion='0.1.0'\nedition='2021'\n[workspace]\n")
    (directory / 'src/lib.rs').write_text(source)
    env = dict(os.environ, RSTYLE_DRIVER=str(bins / 'rstyle-driver'))
    for variable in ['CARGO_TARGET_DIR', 'RSTYLE_PERF']:
        env.pop(variable, None)
    samples = []
    for iteration in range(5):
        for trace in [False, True]:
            tag = f'{phase}-{profile}-{name}-{iteration}-{trace}'
            use = dict(env)
            if iteration == 4:
                use['RSTYLE_PERF'] = '1'
            command = [bins / 'cargo-rstyle', 'clones', '--format', 'json', '--offline',
                       '--extraction-profile', selection] + (['--trace'] if trace else [])
            report, cost = run(command, directory, output, tag, use)
            actual = signature(report, count)
            expected = output / f'{profile}-{name}-signature.json'
            if expected.exists():
                assert json.loads(expected.read_text()) == actual, tag
            else:
                expected.write_text(json.dumps(actual))
            cost.update(phase=phase, profile=profile, workload=name, iteration=iteration,
                        trace=trace, diagnostic=iteration == 4, report_bytes=(output / (tag + '.json')).stat().st_size,
                        **library_inventory(report))
            samples.append(cost)
    replay = bins / 'unit_import_replay'
    if not replay.exists():
        replay = bins / 'examples/unit_import_replay'
    bundle = report['functions'][0]['unit_evidence']['bundle']
    for iteration in range(3):
        tag = f'{phase}-{profile}-{name}-strict-{iteration}'
        imported, cost = run([replay, bundle], directory, output, tag, env)
        assert imported['status'] == 'stage-completed'
        assert imported['functions'] == imported['unique_graphs'] == count
        cost.update(phase=phase, profile=profile, workload=name, strict=True, iteration=iteration)
        samples.append(cost)
    return samples


def main():
    phase, root, destination = sys.argv[1:]
    assert phase in ['before', 'after']
    root, output = pathlib.Path(root).resolve(), pathlib.Path(destination).resolve()
    output.mkdir(exist_ok=True, parents=True)
    samples = []
    for profile in ['debug', 'release']:
        for workload in workloads():
            samples.extend(measure_workload(phase, profile, root / profile, output, workload))
            (output / (phase + '-samples.json')).write_text(json.dumps(samples, indent=2) + '\n')
    for name, *_ in workloads():
        assert json.loads((output / f'debug-{name}-signature.json').read_text()) == json.loads(
            (output / f'release-{name}-signature.json').read_text()), name
    for profile in ['debug', 'release']:
        for name, *_ in workloads():
            for trace in [False, True]:
                values = [sample['wall'] for sample in samples if sample.get('trace') == trace
                          and sample['profile'] == profile and sample['workload'] == name
                          and 0 < sample['iteration'] < 4 and not sample.get('strict')]
                print(profile, name, trace, values, statistics.median(values), flush=True)


if __name__ == '__main__':
    main()
