"""Profile complete clones commands and strict imports after building the tools.

Use --profile release for optimized tools; build time is intentionally excluded.
Artifacts and failures are retained in the printed temporary directory.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--profile', choices=['debug', 'release'], default='debug')
args = parser.parse_args()
repo = pathlib.Path(__file__).resolve().parent.parent
output = pathlib.Path(tempfile.mkdtemp(prefix='rstyle-production-profile-'))
binaries = repo / 'target' / args.profile
print(str(output), flush=True)
source = (repo / 'crates/cargo-rstyle/tests/cli.rs').read_text()
inherent = re.search('const DATA_INHERENT_FAMILY: &str = r#"(.*?)"#;', source, re.S).group(1)
read = json.loads(re.search('const DATA_READ_CALL_FAMILY: &str = (".*?");', source).group(1))
workloads = [
    ('minimal', 'r0', 'pub fn first(a:i32)->i32 {a}\npub fn second(a:i32)->i32 {a}\n', 2, 1),
    ('read', 'hir-data-v2', read, 20, 190),
    ('inherent', 'hir-data-v3', inherent, 21, 210),
]
env = dict(
    os.environ,
    RSTYLE_DRIVER=str(binaries / 'rstyle-driver'),
    RUSTUP_TOOLCHAIN='nightly-2026-09-30',
)
for name in ['CARGO_TARGET_DIR', 'RSTYLE_PERF']:
    env.pop(name, None)
samples = []
baselines = {}


def run(command, directory, tag, diagnostic=False):
    stdout = output / (tag + '.json')
    stderr = output / (tag + '.err')
    timing = output / (tag + '.time')
    use_env = dict(env)
    if diagnostic:
        use_env['RSTYLE_PERF'] = '1'
    with stdout.open('w') as out, stderr.open('w') as err:
        p = subprocess.run(
            ['/usr/bin/time', '-f', '%e %M', '-o', str(timing), 'timeout', '120', *command],
            cwd=directory, env=use_env, stdout=out, stderr=err,
        )
    assert p.returncode == 0, (tag, p.returncode, stderr.read_text()[-3000:])
    wall, rss = timing.read_text().split()
    result = dict(
        tag=tag, wall_seconds=float(wall), rss_kib=int(rss),
        diagnostic=diagnostic, stdout_bytes=stdout.stat().st_size,
    )
    return json.loads(stdout.read_text()), result


for name, profile, text, count, pairs in workloads:
    directory = output / name
    (directory / 'src').mkdir(parents=True)
    (directory / 'Cargo.toml').write_text("[package]\nname='profile_fixture'\nversion='0.1.0'\nedition='2021'\n[workspace]\n")
    (directory / 'src/lib.rs').write_text(text)
    subprocess.run(
        ['cargo', 'generate-lockfile', '--offline'], cwd=directory,
        env=env, check=True, capture_output=True,
    )
    base = [
        str(binaries / 'cargo-rstyle'), 'clones', '--format', 'json',
        '--locked', '--offline', '--extraction-profile', profile,
    ]
    sequence = [('first', False, False)]
    sequence += [
        (f'warm-{sample}-{mode}', trace, False)
        for sample in range(3) for mode, trace in [('plain', False), ('trace', True)]
    ]
    sequence += [('diagnostic-plain', False, True), ('diagnostic-trace', True, True)]
    for mode, trace, diagnostic in sequence:
        report, result = run(base + (['--trace'] if trace else []), directory, name + '-' + mode, diagnostic)
        complete = [
            'capture_complete', 'assessment_complete',
            'pair_enumeration_complete', 'pair_evaluation_complete',
        ]
        assert all(report[k] is True for k in complete), result
        assert len(report['functions']) == count and len(report['matching']['pairs']) == pairs, result
        assert all((f['outcome'] == 'admitted' for f in report['functions'])), result
        signature = [
            (p['first'], p['second'], p['decision'], p.get('displayed_score'))
            for p in report['matching']['pairs']
        ]
        if name in baselines:
            assert signature == baselines[name], result
        else:
            baselines[name] = signature
        result.update(
            workload=name, trace=trace, functions=count, pairs=pairs,
            accepted=sum(p['decision'].lower() == 'accepted' for p in report['matching']['pairs']),
        )
        samples.append(result)
        print(json.dumps(result), flush=True)
        bundle = report['functions'][0]['unit_evidence']['bundle']
    for sample in range(4):
        diagnostic = sample == 3
        report, result = run(
            [str(binaries / 'examples/unit_import_replay'), bundle],
            directory, f'{name}-strict-{sample}', diagnostic,
        )
        assert report['functions'] == count and report['unique_graphs'] == count, result
        assert report['status'] == 'stage-completed', result
        result.update(
            workload=name, strict_import=True, functions=count,
            bundle_bytes=report['bundle_bytes'],
        )
        samples.append(result)
        print(json.dumps(result), flush=True)
(output / 'samples.json').write_text(json.dumps(samples, indent=2))
