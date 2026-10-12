#!/usr/bin/env python3
"""Frozen CCGraph v1 evaluator. No detector policy is implemented here."""
import argparse
import collections
import hashlib
import itertools
import json
import os
import pathlib
import signal
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
STUDY = ROOT / 'eval/ccgraph-v1'
CONTRACT = '04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f'
FLAGS = ['capture_complete', 'assessment_complete', 'pair_enumeration_complete', 'pair_evaluation_complete']
DEFAULTS = {'minimum_size_ratio': '4/5', 'numerical_threshold': '9/10', 'string_threshold': '1/2', 'final_threshold': '9/10'}

class Invalid(ValueError):
    pass

def require(condition, reason):
    if not condition:
        raise Invalid(reason)

def digest(data):
    return hashlib.sha256(data).hexdigest()

def identity(value):
    return digest(json.dumps(value, sort_keys=True, separators=(',', ':')).encode())

def read(path):
    return json.loads(pathlib.Path(path).read_text())

def write(path, value):
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')

def validate_freeze(root=ROOT, study=STUDY):
    frozen = read(study / 'freeze.json')
    payload = {k: v for k, v in frozen.items() if k != 'identity'}
    require(identity(payload) == frozen['identity'], 'freeze identity mismatch')
    for path, expected in frozen['files'].items():
        require((root / path).is_file(), f'missing frozen file: {path}')
        require(digest((root / path).read_bytes()) == expected, f'input/configuration drift: {path}')
    validate_manifest(read(study / 'manifest.json'), study)
    return frozen

def validate_manifest(manifest, study=None):
    scopes = {s['id']: s for s in manifest['scopes']}
    require(len(scopes) == len(manifest['scopes']), 'duplicate scope')
    ids = {f['id']: f for f in manifest['functions']}
    require(len(ids) == len(manifest['functions']), 'duplicate function identity')
    groups = collections.defaultdict(set)
    navigation = set()
    for f in ids.values():
        require(f['scope'] in scopes, 'unknown function scope')
        scope = scopes[f['scope']]
        for group in (f['family'], f['provenance_group']):
            groups[group].add(scope['split'])
        nav = (f['scope'], f['package'], f['target'], f['path'], f['start'], f['end'])
        require(nav not in navigation, 'ambiguous source navigation')
        navigation.add(nav)
        require(f['start'] < f['end'], 'invalid byte span')
        if study:
            data = (study / f['path']).read_bytes()[f['start']:f['end']]
            require(digest(data) == f['sha256'], 'source digest mismatch')
    require(all(len(splits) == 1 for splits in groups.values()), 'family/project/context split leakage')
    pairs = set()
    for label in manifest['labels']:
        a, b = label['first'], label['second']
        require(a in ids and b in ids and a != b, 'invalid label functions')
        require(ids[a]['scope'] == ids[b]['scope'] == label['scope'], 'out-of-scope label')
        key = tuple(sorted((a, b)))
        require(key not in pairs, 'duplicate label')
        require(label['label'] in ('positive', 'negative', 'ambiguous', 'unlabeled'), 'invalid label')
        require(bool(label['rationale'].strip()), 'missing source rationale')
        pairs.add(key)
    expected = set()
    for scope in scopes.values():
        fs = [f['id'] for f in ids.values() if f['scope'] == scope['id']]
        require(len(fs) == scope['source_functions'], 'selected source count mismatch')
        expected.update(tuple(sorted(p)) for p in itertools.combinations(fs, 2))
    require(pairs == expected, 'missing/extra source pair labels')

def mapping(report, scope, functions, study=STUDY):
    units = {str(pathlib.Path(u['directory'])): u for u in report['observed_units']}
    require(len(units) == len(report['observed_units']) == scope['selected_units'], 'duplicate/missing compilation units')
    require(len(report['intended_primary_units']) == scope['selected_units'], 'planned compilation unit count mismatch')
    selected = [f for f in functions if f['scope'] == scope['id']]
    result = {}; observations = set()
    for record in report['functions']:
        key = record['key']
        require(key not in result, 'duplicate reported function key')
        unitpath = str(pathlib.Path(record['artifact_prefix']).parent)
        require(unitpath in units, 'function outside observed compilation context')
        unit = units[unitpath]; target = unit['target']; facts = record['facts']
        options = facts['build']['frontend_options']; span = facts['fn_span']
        matches = [f for f in selected if
                   pathlib.Path(options['source_file']).resolve() == (study / f['path']).resolve()
                   and span == {'start': f['start'], 'end': f['end']}
                   and pathlib.Path(target['manifest_dir']).resolve() == (study / f['path']).parents[1].resolve()
                   and target['name'] == f['target'] and target['kind'] == ['lib']]
        if not matches and span == {'start': 0, 'end': 0} and facts['source']['bytes'] == '' and unit['test'] is True:
            require(record['outcome'] == 'excluded' and facts.get('source_scan_complete') is False
                    and any('Macro-generated functions' in reason for reason in record['reasons']), 'unexpected generated function')
            require(pathlib.Path(options['source_file']).resolve() == pathlib.Path(target['source']).resolve(), 'generated function outside target source')
            generated_id = 'compiler-generated:' + target['package'] + ':' + target['name']
            require(not any(f['id'] == generated_id for f in result.values()), 'duplicate generated entrypoint')
            result[key] = {'id': generated_id, 'test': True, 'family': 'compiler-generated', 'provenance_group': 'compiler-generated',
                           'outcome': record['outcome'], 'reasons': record['reasons'], 'key': key, 'selected_source': False}
            continue
        require(len(matches) == 1, 'ambiguous/missing function mapping')
        f = matches[0]; context = (f['id'], unit['test'])
        require(type(unit['test']) is bool and context not in observations, 'duplicate compilation context mapping')
        observations.add(context)
        require(digest(facts['source']['bytes'].encode()) == f['sha256'], 'mapped source bytes mismatch')
        require(facts['source']['digest'] == 'sha256:' + f['sha256'], 'mapped source digest mismatch')
        require(record['outcome'] in ('admitted', 'excluded', 'unknown'), 'failed/unclassified function')
        result[key] = {'id': f['id'], 'test': unit['test'], 'family': f['family'], 'provenance_group': f['provenance_group'],
                       'outcome': record['outcome'], 'reasons': record['reasons'], 'key': key, 'selected_source': True}
    expected = {(f['id'], test) for f in selected for test in (False, True)}
    require(observations == expected, 'missing/extra selected function contexts')
    return result

def verify_pairs(report, mapped):
    admitted = {k for k, f in mapped.items() if f['outcome'] == 'admitted'}
    expected = {tuple(sorted(p)) for p in itertools.combinations(admitted, 2)}
    seen = {}; rows = report['matching']['pairs']
    require({f['key'] for f in report['matching']['functions']} == admitted, 'matching function inventory mismatch')
    require(len(report['matching']['functions']) == len(admitted), 'duplicate matching function')
    for pair in rows:
        key = tuple(sorted((pair['first'], pair['second'])))
        require(key in expected and key not in seen, 'duplicate/out-of-scope pair')
        status = pair['evaluation_status']
        require(status in ('evaluated', 'incompatible'), 'execution/arithmetic pair failure')
        require(len(pair['stages']) == 7, 'incomplete stage inventory')
        require([s['stage'] for s in pair['stages']] == ['Validation', 'Projection', 'Nonempty', 'SizeRatio', 'Numerical', 'Strings', 'WlFinal'], 'malformed stage order')
        require(not any(isinstance(s['outcome'], dict) and 'Error' in s['outcome'] for s in pair['stages']), 'arithmetic/stage error')
        if status == 'evaluated':
            require(pair['decision'] in ('Accepted', 'Rejected'), 'missing evaluated decision')
            rejected = any(isinstance(s['outcome'], dict) and 'Rejected' in s['outcome'] for s in pair['stages'])
            passed = all(s['outcome'] == 'Passed' for s in pair['stages'])
            require((pair['decision'] == 'Accepted' and passed) or (pair['decision'] == 'Rejected' and rejected), 'decision/stage contradiction')
        else:
            require(pair['decision'] is None and any(isinstance(s['outcome'], dict) and 'Ineligible' in s['outcome'] for s in pair['stages']), 'incompatible decision contradiction')
        seen[key] = pair
    require(set(seen) == expected, 'missing pair inventory')
    return seen

def ratio(n, d):
    return n / d if d else None

def metrics(rows):
    c = collections.Counter(); labels = collections.Counter(); non = collections.Counter()
    positive_admitted = 0; accepted_unlabeled = 0
    groups = set()
    for row in rows:
        label = row['label']; labels[label] += 1
        if label == 'positive' and row['both_admitted']:
            positive_admitted += 1
        if row['status'] != 'evaluated':
            non[row['status']] += 1
            continue
        accepted = row['decision'] == 'Accepted'
        if label in ('ambiguous', 'unlabeled'):
            accepted_unlabeled += accepted
            continue
        groups.update(row['groups'])
        c[('TP' if accepted else 'FN') if label == 'positive' else ('FP' if accepted else 'TN')] += 1
    tp, fp, tn, fn = (c[x] for x in ('TP', 'FP', 'TN', 'FN'))
    return {'selected_pairs': len(rows), 'labels': dict(sorted(labels.items())), 'non_evaluable': dict(sorted(non.items())),
            'evaluable_positive': tp + fn, 'evaluable_negative': fp + tn, 'independent_groups': sorted(groups),
            'TP': tp, 'FP': fp, 'TN': tn, 'FN': fn, 'precision': ratio(tp, tp + fp), 'recall': ratio(tp, tp + fn),
            'false_positive_rate': ratio(fp, fp + tn), 'f1': ratio(2 * tp, 2 * tp + fp + fn),
            'positive_admission_coverage': ratio(positive_admitted, labels['positive']),
            'positive_evaluability': ratio(tp + fn, labels['positive']),
            'end_to_end_positive_yield': ratio(tp, labels['positive']), 'unlabeled_accepted': accepted_unlabeled}

def targets(m):
    sufficient = m['evaluable_positive'] >= 15 and m['evaluable_negative'] >= 30 and len(m['independent_groups']) >= 3
    criteria = {'precision': (0.90, True), 'recall': (0.80, True), 'false_positive_rate': (0.10, False),
                'positive_admission_coverage': (0.80, True), 'end_to_end_positive_yield': (0.64, True)}
    def outcome(value, threshold, higher):
        if value is None: return 'insufficient evidence'
        return 'met' if (value >= threshold if higher else value <= threshold) else 'not met'
    return {name: {'threshold': threshold, 'value': m[name],
                   'outcome': outcome(m[name], threshold, higher) if sufficient else 'insufficient evidence',
                   'descriptive_numerical_outcome': outcome(m[name], threshold, higher)}
            for name, (threshold, higher) in criteria.items()}

def assess(report, scope, manifest, variant, study=STUDY):
    require(all(report.get(f) is True for f in FLAGS), 'incomplete capture/assessment/enumeration/evaluation')
    require(not report['matching']['incomplete'], 'incomplete matching execution')
    require(not report['capture_failures'] and not report['handoff_failures'] and not report['matching']['diagnostics'], 'execution diagnostics')
    require(report['contract_revision'] == CONTRACT, 'contract drift')
    require(report['matching']['profile_id'] == 'compat.ccgraph.strict' and report['matching']['profile_version'] == '0.1.0', 'profile drift')
    scoring = 'smaller_graph_overlap' if variant == 'default' else 'cross_product_cosine'
    require(report['matching']['scoring'] == scoring, 'scoring drift')
    require(report['matching']['effective_config'] == dict(DEFAULTS, scoring=scoring), 'effective config drift')
    mapped = mapping(report, scope, manifest['functions'], study)
    pairs = verify_pairs(report, mapped)
    byid = {f['id']: f for f in mapped.values() if f['test'] is False}
    rows = []
    for label in manifest['labels']:
        if label['scope'] != scope['id']: continue
        a, b = byid[label['first']], byid[label['second']]
        both = a['outcome'] == b['outcome'] == 'admitted'
        pair = pairs.get(tuple(sorted((a['key'], b['key']))))
        status = pair['evaluation_status'] if pair else '+'.join(sorted(set(f['outcome'] for f in (a, b) if f['outcome'] != 'admitted')))
        rows.append(dict(label, status=status, decision=pair['decision'] if pair else None, both_admitted=both,
                         detector_keys=[a['key'], b['key']], reasons=[a['reasons'], b['reasons']],
                         groups=sorted(set(f['provenance_group'] for f in (a, b)))))
    rows.sort(key=lambda r: (r['first'], r['second']))
    allpairs = []
    for key, pair in sorted(pairs.items()):
        allpairs.append(dict(pair, study_first=mapped[pair['first']]['id'], study_second=mapped[pair['second']]['id'],
                             first_test=mapped[pair['first']]['test'], second_test=mapped[pair['second']]['test']))
    m = metrics(rows)
    result = {'scope': scope, 'variant': variant, 'valid': True,
              'units': {'selected': scope['selected_units'], 'observed': len(report['observed_units'])},
              'functions': {'selected': 2 * scope['source_functions'], 'observed': len(mapped), 'assessed': len(mapped),
                            'selected_source_observed': sum(f['selected_source'] for f in mapped.values()),
                            'compiler_generated_observed': sum(not f['selected_source'] for f in mapped.values()),
                            'selected_source_outcomes': dict(sorted(collections.Counter(f['outcome'] for f in mapped.values() if f['selected_source']).items())),
                            'outcomes': dict(sorted(collections.Counter(f['outcome'] for f in mapped.values()).items()))},
              'declared_context_pair_universe': (2 * scope['source_functions']) * (2 * scope['source_functions'] - 1) // 2,
              'observed_function_pair_universe': len(mapped) * (len(mapped) - 1) // 2,
              'emitted_pairs': len(pairs), 'evaluated_pairs': sum(p['evaluation_status'] == 'evaluated' for p in pairs.values()),
              'incompatible_pairs': sum(p['evaluation_status'] == 'incompatible' for p in pairs.values()),
              'metrics': m, 'targets': targets(m), 'canonical_pairs': rows,
              'strata': {s: metrics([r for r in rows if r['stratum'] == s]) for s in sorted({r['stratum'] for r in rows})},
              'groups': {g: metrics([r for r in rows if g in r['groups']]) for g in sorted({g for r in rows for g in r['groups']})},
              'mapping': sorted(mapped.values(), key=lambda f: (f['id'], f['test'])), 'pairs': allpairs,
              'matching_functions': sorted(report['matching']['functions'], key=lambda f: f['key'])}
    return result

def binaries():
    paths = {name: ROOT / 'target/debug' / name for name in ('cargo-rstyle', 'rstyle-driver')}
    require(all(p.is_file() for p in paths.values()), 'build the pinned binaries first')
    return {name: digest(path.read_bytes()) for name, path in paths.items()}

def tree_rss(pid):
    """Linux proc sampled concurrent process-group RSS, bytes; null if unavailable."""
    try:
        total = 0
        for p in pathlib.Path('/proc').iterdir():
            if not p.name.isdigit(): continue
            try:
                stat = (p / 'stat').read_text().rsplit(')', 1)[1].split()
                if int(stat[2]) == pid: total += int(stat[21]) * os.sysconf('SC_PAGE_SIZE')
            except (OSError, ValueError, IndexError): pass
        return total
    except OSError:
        return None

def execute(scope, variant, destination):
    destination.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ); env['PATH'] = str(ROOT / 'target/debug') + ':/home/dev/.cargo/bin:' + env.get('PATH', '')
    command = ['cargo', 'rstyle', 'clones', '--format', 'json', '--all-targets', '--locked', '--offline', '--jobs', '2',
               '--max-pairs', '1000', '--trace', '--matching-config', str(STUDY / (variant + '.json')),
               '--manifest-path', str(STUDY / scope['manifest'])]
    actual = ['/usr/bin/time', '-f', '%U %S %M', '-o', str(destination / 'time.txt')] + command
    start = time.monotonic(); peak = None; failure = None
    with (destination / 'stdout.json').open('w') as out, (destination / 'stderr.txt').open('w') as err:
        process = subprocess.Popen(actual, cwd=ROOT, env=env, stdout=out, stderr=err, start_new_session=True)
        while process.poll() is None:
            rss = tree_rss(process.pid)
            if rss is not None: peak = max(peak or 0, rss)
            if time.monotonic() - start > 180: failure = 'wall budget exceeded'
            if peak is not None and peak > 2 * 1024**3: failure = 'process-tree RSS budget exceeded'
            if failure:
                os.killpg(process.pid, signal.SIGKILL); process.wait(); break
            time.sleep(0.05)
    cost = {'wall_seconds': time.monotonic() - start, 'sampled_process_group_peak_rss_bytes': peak,
            'rss_sampling_seconds': 0.05, 'command': command, 'exit_code': process.returncode,
            'phase_timing': 'combined compilation/capture/matching; command has no separate phase timers'}
    if (destination / 'time.txt').exists():
        fields = (destination / 'time.txt').read_text().splitlines()[-1].split()
        if len(fields) == 3:
            cost.update(user_seconds=float(fields[0]), system_seconds=float(fields[1]), max_child_rss_kib=int(fields[2]))
    write(destination / 'cost.json', cost)
    require(not failure, failure or '')
    require(process.returncode == 0, 'real command failed; raw output retained')
    report = read(destination / 'stdout.json')
    directory = pathlib.Path(report['run_directory'])
    require(directory.is_dir(), 'missing fresh run directory')
    size = sum(p.stat().st_size for p in directory.rglob('*') if p.is_file())
    cost['fresh_run_bytes'] = size
    require(size <= 1024**3, 'fresh-run disk budget exceeded')
    require(digest((directory / 'report.json').read_bytes()) == digest((destination / 'stdout.json').read_bytes().rstrip(b'\n')), 'stdout/stored report drift')
    inventory = {str(p.relative_to(directory)): digest(p.read_bytes()) for p in directory.rglob('*')
                 if p.is_file() and 'cargo-target' not in p.parts}
    write(destination / 'evidence-inventory.json', inventory)
    return report, cost, identity(inventory)

def snapshot():
    frozen = validate_freeze()
    # Production tree must equal the evaluated tree, even as study commits advance.
    diff = subprocess.run(['git', 'diff', frozen['detector_revision'], '--', 'crates', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'rstyle.toml', 'rstyle-ignore.toml'], cwd=ROOT, capture_output=True, check=True)
    require(not diff.stdout, 'production source/settings drift')
    return {'protocol_identity': frozen['identity'], 'binaries': binaries(),
            'compiler': subprocess.check_output(['/home/dev/.cargo/bin/rustup', 'run', 'nightly-2026-09-30', 'rustc', '-Vv'], text=True),
            'cargo': subprocess.check_output(['/home/dev/.cargo/bin/rustup', 'run', 'nightly-2026-09-30', 'cargo', '-Vv'], text=True),
            'detector_revision': frozen['detector_revision']}

def run_split(split, smoke=False):
    manifest = read(STUDY / 'manifest.json'); snap = snapshot()
    stage = 'smoke' if smoke else split
    if split == 'heldout':
        require(read(STUDY / 'results/development-binary-freeze.json') == snap, 'binary/config drift before heldout')
        require(all((STUDY / 'results' / f"{s['id']}-{v}.json").exists() for s in manifest['scopes'] if s['split'] == 'development' for v in ('default', 'cosine')), 'development not complete')
    scopes = [s for s in manifest['scopes'] if s['split'] == split]
    if smoke: scopes = [scopes[0]]
    stamp = time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
    for scope in scopes:
        for variant in (('default',) if smoke else ('default', 'cosine')):
            name = f"{scope['id']}-{variant}"
            output = STUDY / 'results' / (stage + '-' + name + '.json' if smoke else name + '.json')
            require(not output.exists(), 'refusing overwrite of preserved evaluation')
            raw = STUDY / 'artifacts' / (stage + '-' + name + '-' + stamp)
            try:
                report, cost, inventory = execute(scope, variant, raw)
                require(snapshot() == snap, 'binary/configuration drift during execution')
                require(report['producer_binary_sha256'] == snap['binaries']['rstyle-driver'] and report['orchestration_binary_sha256'] == snap['binaries']['cargo-rstyle'], 'reported binary drift')
                result = assess(report, scope, manifest, variant)
                result.update(identities=snap, cost=cost, raw_report_sha256=digest((raw / 'stdout.json').read_bytes()),
                              evidence_inventory_identity=inventory, raw_directory=str(raw.relative_to(ROOT)),
                              study_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip())
                write(output, result)
                print(name, json.dumps(result['metrics'], sort_keys=True), flush=True)
            except (Invalid, OSError, KeyError, ValueError) as error:
                write(raw / 'invalid.json', {'valid': False, 'reason': str(error), 'identities': snap, 'retry_requires_documented_cause': True})
                raise Invalid(f'{name}: {error}; retained {raw}') from error
    if split == 'development' and not smoke:
        write(STUDY / 'results/development-binary-freeze.json', snap)

def summarize():
    manifest = read(STUDY / 'manifest.json'); summaries = []
    for s in manifest['scopes']:
        for v in ('default', 'cosine'):
            result = read(STUDY / 'results' / f"{s['id']}-{v}.json")
            require(result['valid'], 'invalid saved result')
            summaries.append({k: result[k] for k in ('scope', 'variant', 'units', 'functions', 'declared_context_pair_universe', 'emitted_pairs', 'evaluated_pairs', 'incompatible_pairs', 'metrics', 'strata', 'groups', 'targets', 'cost', 'identities', 'raw_report_sha256', 'evidence_inventory_identity')})
    write(STUDY / 'results/summary.json', summaries)
    primary = [s for s in summaries if s['scope']['split'] == 'heldout' and s['variant'] == 'default']
    return 0 if all(t['outcome'] == 'met' for s in primary for t in s['targets'].values()) else 1

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['validate', 'smoke', 'development', 'heldout', 'summarize'])
    args = parser.parse_args()
    try:
        validate_freeze()
        if args.action == 'validate': return 0
        if args.action == 'summarize': return summarize()
        run_split('development' if args.action == 'smoke' else args.action, args.action == 'smoke')
        return 0  # Execution-only subcommands; target check is summarize.
    except (Invalid, OSError, KeyError, ValueError) as error:
        print(f'INVALID: {error}', file=sys.stderr)
        return 2

if __name__ == '__main__':
    sys.exit(main())
