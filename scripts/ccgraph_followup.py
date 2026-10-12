#!/usr/bin/env python3
"""CCGraph v2: fresh real-command coverage/correctness study, without detector policy."""
import argparse
import collections
import json
import pathlib
import subprocess
import sys
import time

import ccgraph_study as base

STUDY = base.ROOT / 'eval/ccgraph-v2'
BIN_DIR = STUDY / 'artifacts/frozen-bin'


def validate():
    frozen = base.validate_freeze(base.ROOT, STUDY)
    manifest = base.read(STUDY / 'manifest.json')
    oracles = base.read(STUDY / 'oracles.json')
    expected = {f['id'] for f in manifest['functions'] if f['expected_outcome'] == 'admitted'}
    base.require(len(oracles) == len(expected) and {o['id'] for o in oracles} == expected, 'oracle inventory mismatch')
    for f in manifest['functions']:
        base.require(f['expected_outcome'] in ('admitted', 'excluded', 'unknown'), 'invalid source expectation')
        base.require(bool(f['expectation_rationale'].strip()), 'missing expectation rationale')
    for p in base.read(STUDY / 'provenance.json'):
        data = (STUDY / 'upstream' / p['function'].split(':')[0] / p['revision'] / p['path']).read_bytes()
        base.require(base.digest(data) == p['source_sha256'], 'upstream drift')
        source = next(f for f in manifest['functions'] if f['id'] == p['function'])
        excerpt = data[p['excerpt_start']:p['excerpt_end']]
        base.require(base.digest(excerpt) == p['excerpt_sha256'] == source['sha256'], 'excerpt adaptation drift')
        for license in p['licenses']:
            base.require(base.digest((STUDY / license['path']).read_bytes()) == license['sha256'], 'license drift')
    return frozen


def snapshot():
    frozen = validate()
    diff = subprocess.check_output(['git', 'diff', frozen['detector_revision'], '--', 'crates', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'rstyle.toml', 'rstyle-ignore.toml'], cwd=base.ROOT)
    base.require(not diff, 'production source/settings drift')
    paths = {n: BIN_DIR / n for n in ('cargo-rstyle', 'rstyle-driver')}
    base.require(all(p.is_file() for p in paths.values()), 'missing isolated binaries')
    return {'protocol_identity': frozen['identity'], 'binaries': {n: base.digest(p.read_bytes()) for n, p in paths.items()},
            'compiler': subprocess.check_output(['/home/dev/.cargo/bin/rustup', 'run', 'nightly-2026-09-30', 'rustc', '-Vv'], text=True),
            'cargo': subprocess.check_output(['/home/dev/.cargo/bin/rustup', 'run', 'nightly-2026-09-30', 'cargo', '-Vv'], text=True),
            'detector_revision': frozen['detector_revision'],
            'harness_sha256': base.digest(pathlib.Path(__file__).read_bytes()),
            'amendment_sha256': base.digest((STUDY / 'AMENDMENTS.md').read_bytes())}


def compare_oracle(document, oracle):
    counts = collections.Counter(n['category'] for n in document['nodes'])
    ops = sorted(n['attributes']['operator']['value'] for n in document['nodes']
                 if n.get('attributes', {}).get('operator', {}).get('status') == 'known')
    expected_counts = oracle['counts']
    actual = {k: counts[k] for k in expected_counts}
    return {'expected_counts': expected_counts, 'actual_counts': actual,
            'expected_operators': oracle['operators'], 'actual_operators': ops,
            'agrees': actual == expected_counts and ops == oracle['operators']}


def correctness(report, result, manifest):
    oracles = {o['id']: o for o in base.read(STUDY / 'oracles.json')}
    functions = {f['id']: f for f in manifest['functions']}
    mapped = {f['key']: f for f in result['mapping'] if f['selected_source']}
    rows = []
    for record in report['functions']:
        if record['key'] not in mapped:
            continue
        m = mapped[record['key']]; f = functions[m['id']]
        row = {'id': f['id'], 'test': m['test'], 'expected': f['expected_outcome'], 'actual': m['outcome'],
               'outcome_agrees': f['expected_outcome'] == m['outcome'], 'reasons': m['reasons'], 'oracle': None}
        if f['id'] in oracles and m['outcome'] == 'admitted':
            path = pathlib.Path(record['artifact_prefix']).with_suffix('.pdg.json')
            data = path.read_bytes(); document = json.loads(data)
            entry = next(d for d in record['document_manifest']['documents'] if d['key'] == document['extensions']['compat.rust.hir-r0:paired-graph']['pdg_document_key'])
            base.require(base.digest(data) == entry['sha256'].removeprefix('sha256:'), 'oracle document digest drift')
            row['oracle'] = compare_oracle(document, oracles[f['id']])
            row['pdg_sha256'] = base.digest(data)
        rows.append(row)
    return sorted(rows, key=lambda r: (r['id'], r['test']))


def write(path, value):
    """Compact stable JSON: retain every pair/stage without v1's pretty-print bulk."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n')


def run(split, smoke=False):
    manifest = base.read(STUDY / 'manifest.json'); snap = snapshot()
    if split == 'heldout':
        base.require(base.read(STUDY / 'results/development-binary-freeze.json') == snap, 'binary/config drift before heldout')
        base.require(all((STUDY / 'results' / f"{s['id']}-{v}.json").exists() for s in manifest['scopes'] if s['split'] == 'development' for v in ('default', 'cosine')), 'development not complete')
    scopes = [s for s in manifest['scopes'] if s['split'] == split]
    if smoke:
        scopes = [s for s in scopes if s['id'] == 'audit']
    # Explicit execution context for the shared command runner only. No cached evidence.
    base.STUDY = STUDY; base.BIN_DIR = BIN_DIR
    stamp = time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
    for scope in scopes:
        for variant in (('default',) if smoke else ('default', 'cosine')):
            name = ('smoke-' if smoke else '') + scope['id'] + '-' + variant
            output = STUDY / 'results' / (name + '.json')
            base.require(not output.exists(), 'refusing overwrite of preserved evaluation')
            raw = STUDY / 'artifacts' / (name + '-' + stamp)
            try:
                report, cost, inventory = base.execute(scope, variant, raw)
                base.require(snapshot() == snap, 'binary/configuration drift during execution')
                base.require(report['producer_binary_sha256'] == snap['binaries']['rstyle-driver'] and report['orchestration_binary_sha256'] == snap['binaries']['cargo-rstyle'], 'reported binary drift')
                result = base.assess(report, scope, manifest, variant, STUDY)
                result['targets'] = quality_targets(result['metrics'])
                result['correctness'] = correctness(report, result, manifest)
                result.update(identities=snap, cost=cost, raw_report_sha256=base.digest((raw / 'stdout.json').read_bytes()),
                              evidence_inventory_identity=inventory, raw_directory=str(raw.relative_to(base.ROOT)),
                              study_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=base.ROOT, text=True).strip())
                result['report_identity'] = {k: report[k] for k in ('toolchain', 'compiler_identity', 'cargo_identity', 'contract_revision', 'extraction_policy', 'schema_versions', 'scope')}
                result['producer_build_revisions'] = sorted({f['facts']['build']['producer_revision']['value'] for f in report['functions']})
                write(output, result)
                print(name, json.dumps({'metrics': result['metrics'], 'correctness': result['correctness']}, sort_keys=True), flush=True)
            except (base.Invalid, OSError, KeyError, ValueError, StopIteration) as error:
                base.write(raw / 'invalid.json', {'valid': False, 'reason': str(error), 'identities': snap, 'retry_requires_documented_cause': True})
                raise base.Invalid(f'{name}: {error}; retained {raw}') from error
    if split == 'development' and not smoke:
        base.write(STUDY / 'results/development-binary-freeze.json', snap)


def target(numerator, denominator, minimum, projects, minimum_projects=1):
    sufficient = denominator >= minimum and projects >= minimum_projects
    return {'numerator': numerator, 'denominator': denominator, 'value': base.ratio(numerator, denominator),
            'minimum': minimum, 'projects': projects, 'minimum_projects': minimum_projects, 'threshold': 1,
            'outcome': 'insufficient evidence' if not sufficient else 'met' if numerator == denominator else 'not met'}


def quality_targets(metrics):
    sufficient = metrics['evaluable_positive'] >= 20 and metrics['evaluable_negative'] >= 20 and len(metrics['independent_groups']) >= 2
    criteria = {'precision': (.95, True), 'recall': (.80, True), 'false_positive_rate': (.05, False),
                'f1': (.85, True), 'positive_admission_coverage': (.80, True), 'end_to_end_positive_yield': (.70, True)}
    result = {}
    for name, (threshold, higher) in criteria.items():
        value = metrics[name]
        outcome = 'insufficient evidence' if value is None else 'met' if (value >= threshold if higher else value <= threshold) else 'not met'
        result[name] = {'value': value, 'threshold': threshold, 'outcome': outcome if sufficient else 'insufficient evidence',
                        'descriptive_numerical_outcome': outcome}
    return result


def coverage_targets(results):
    natural = [r for r in results if r['scope']['population'] == 'natural' and r['scope']['split'] == 'heldout' and r['variant'] == 'default']
    rows = [dict(c, project=r['scope']['id']) for r in natural for c in r['correctness'] if not c['test']]
    supported = [r for r in rows if r['expected'] == 'admitted']
    excluded = [r for r in rows if r['expected'] == 'excluded']
    unknown = [c for r in results if r['scope']['id'] == 'audit' and r['variant'] == 'default' for c in r['correctness'] if not c['test'] and c['expected'] == 'unknown']
    return {'natural_heldout_admission': target(sum(r['actual'] == 'admitted' for r in supported), len(supported), 3, len({r['project'] for r in supported}), 2),
            'natural_heldout_bounded_oracles': target(sum(r['oracle'] is not None and r['oracle']['agrees'] for r in supported), len(supported), 3, len({r['project'] for r in supported}), 2),
            'natural_heldout_exclusions': target(sum(r['outcome_agrees'] for r in excluded), len(excluded), 2, len({r['project'] for r in excluded})),
            'development_unknown_controls': target(sum(r['outcome_agrees'] for r in unknown), len(unknown), 2, 1)}


def summarize():
    validate(); manifest = base.read(STUDY / 'manifest.json'); results = []
    for scope in manifest['scopes']:
        for variant in ('default', 'cosine'):
            r = base.read(STUDY / 'results' / f"{scope['id']}-{variant}.json")
            # Shared validator checks v1 target arithmetic; independently check v2 targets.
            base.validate_result(dict(r, targets=base.targets(r['metrics'])), scope, manifest, variant)
            base.require(r['targets'] == quality_targets(r['metrics']), 'saved quality target drift')
            base.require(r['identities'] == base.read(STUDY / 'results/development-binary-freeze.json'), 'saved binary/configuration drift')
            base.require(len(r['correctness']) == 2 * scope['source_functions'], 'saved correctness inventory mismatch')
            expected = {(f['id'], test) for f in manifest['functions'] if f['scope'] == scope['id'] for test in (False, True)}
            base.require({(c['id'], c['test']) for c in r['correctness']} == expected, 'saved correctness mapping drift')
            for c in r['correctness']:
                f = next(f for f in manifest['functions'] if f['id'] == c['id'])
                mapped = next(m for m in r['mapping'] if m['id'] == c['id'] and m['test'] == c['test'])
                base.require(c['expected'] == f['expected_outcome'] and c['actual'] == mapped['outcome'] and c['outcome_agrees'] == (c['expected'] == c['actual']), 'saved outcome drift')
                if c['oracle']:
                    o = next(o for o in base.read(STUDY / 'oracles.json') if o['id'] == c['id']); a = c['oracle']
                    base.require(a['expected_counts'] == o['counts'] and a['expected_operators'] == o['operators'], 'saved oracle drift')
                    base.require(a['agrees'] == (a['actual_counts'] == o['counts'] and a['actual_operators'] == o['operators']), 'saved oracle agreement drift')
                base.require((c['oracle'] is not None) == (f['expected_outcome'] == c['actual'] == 'admitted'), 'missing/extra oracle assessment')
            results.append(r)
    groups = {}
    for population in ('natural', 'controlled'):
        for split in ('development', 'heldout'):
            for variant in ('default', 'cosine'):
                subset = [r for r in results if r['scope']['population'] == population and r['scope']['split'] == split and r['variant'] == variant]
                if not subset: continue
                rows = [p for r in subset for p in r['canonical_pairs']]
                metrics = base.metrics(rows)
                groups[':'.join((population, split, variant))] = {'metrics': metrics, 'secondary_quality_targets': quality_targets(metrics)}
    targets = coverage_targets(results)
    summary = {'primary_targets': targets, 'populations': groups, 'scopes': [{k: r[k] for k in ('scope', 'variant', 'units', 'functions', 'declared_context_pair_universe', 'observed_function_pair_universe', 'emitted_pairs', 'evaluated_pairs', 'incompatible_pairs', 'metrics', 'strata', 'groups', 'correctness', 'cost', 'identities', 'report_identity', 'producer_build_revisions', 'raw_report_sha256', 'evidence_inventory_identity')} for r in results]}
    base.write(STUDY / 'results/summary.json', summary)
    return 0 if all(t['outcome'] == 'met' for t in targets.values()) else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['validate', 'smoke', 'development', 'heldout', 'summarize'])
    action = parser.parse_args().action
    try:
        validate()
        if action == 'validate': return 0
        if action == 'summarize': return summarize()
        run('development' if action == 'smoke' else action, action == 'smoke')
        return 0
    except (base.Invalid, OSError, KeyError, ValueError, StopIteration) as error:
        print(f'INVALID: {error}', file=sys.stderr)
        return 2

if __name__ == '__main__':
    sys.exit(main())
