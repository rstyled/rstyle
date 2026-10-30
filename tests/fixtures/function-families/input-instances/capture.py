#!/usr/bin/env python3
"""Retain owning Cargo reference-instance diagnostics; no admission claim."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess


def load_oracle(directory):
    spec = importlib.util.spec_from_file_location('input_oracle', directory / 'oracle.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def requests():
    # rustc renders the trait's prelude spelling according to the original crate.
    owners = ["util::search::Input::<'h>::new"] + [
        f"<util::search::Input<'h> as {root}::convert::From<&'h H>>::from"
        for root in ['core', 'std']]
    return {'identity': 'rstyle.diagnostic.reference-instance-requests/1',
            'crate_name': 'regex_automata',
            'requests': [{'function': owner, 'haystack': haystack}
                         for owner in owners for haystack in ['[u8]', 'str']]}


def check_observation(entry, selection):
    constructor = entry['request']['function'].endswith('::new')
    expected = selection['functions'][0 if constructor else 1]
    original = entry['original']
    if original['source']['bytes'] != expected['source'] or original['fn_span'] != expected['span']:
        raise ValueError('observed owner differs from the independently selected original source')
    provenance = entry['provenance']
    owners = provenance['owners']
    parameters = [parameter for owner in owners for parameter in owner['parameters']]
    if (provenance['complete'] is not True or len(owners) != 2
            or [(p['index'], p['kind']) for p in parameters] != [(0, 'lifetime'), (1, 'type')]
            or owners[0]['declaration']['span']['start'] >= expected['span']['start']):
        raise ValueError('original impl/binder evidence is incomplete')
    signature = provenance['signature']
    argument = signature['inputs'][0]
    if (argument['kind'] != 'reference' or argument['lifetime']['index'] != 0
            or argument['referent']['index'] != 1
            or signature['output']['arguments'][0]['value']['index'] != 0):
        raise ValueError('original lifetime/type parameter correspondence differs')
    if len(entry['conversions']) != int(constructor) or len(entry['calls']) != int(not constructor):
        raise ValueError('constructor conversion/From call rosters differ')
    if constructor:
        conversion = entry['conversions'][0]
        for key in ['requested', 'requested_trait', 'selected', 'selected_impl', 'selected_trait']:
            if conversion[key]['crate'] != 'core' or conversion[key]['local'] is not False:
                raise ValueError('selected conversion is outside the original core implementations')
        for key in ['requested_signature', 'signature']:
            if conversion[key]['receiver_region'] != conversion[key]['result_region']:
                raise ValueError('selected conversion loses the original borrow binder')


def summarize(directory, completed, selection):
    report = json.loads((directory / 'stdout.json').read_text())
    files = list((directory / 'snapshots').glob('*-reference-instances.json'))
    if len(files) != 1:
        raise ValueError(f'expected one original owning inventory, got {len(files)}')
    inventory = json.loads(files[0].read_text())
    observed = [entry for entry in inventory['requests'] if entry['status'] == 'observed']
    if (len(observed) != 4 or inventory['admission_authority'] is not False
            or report['capture_complete'] is not False or report['functions']
            or completed.returncode == 0):
        raise ValueError('incomplete diagnostics or unexpected successful publication')
    if any(entry['status'] not in ['observed', 'unavailable'] for entry in inventory['requests']):
        raise ValueError('a selected original instance could not be observed')
    for entry in observed:
        check_observation(entry, selection)
    constructors = {entry['request']['haystack']: entry for entry in observed
                    if entry['request']['function'].endswith('::new')}
    for entry in observed:
        if entry['calls']:
            target = constructors[entry['request']['haystack']]['definition']['definition_identity']
            if entry['calls'][0]['selected']['definition_identity'] != target:
                raise ValueError('From does not select the matching original constructor')
    raw = list((directory / 'snapshots').glob('*-pre-helper-certification.json'))
    if len(raw) != 1:
        raise ValueError('missing complete original definition roster')
    roster = json.loads(raw[0].read_text())
    if roster.get('schema') not in ('rstyle.diagnostic.raw-facts/2', 'rstyle.diagnostic.raw-facts/3'):
        raise ValueError('expected shared-source original roster')
    return {'definition_count': len(roster['facts']), 'observed_instances': len(observed),
            'admission_authority': False,
            'instances': [{'request': entry['request'],
                           'owners': len(entry['provenance']['owners']),
                           'nodes': len(entry['nodes']), 'conversions': len(entry['conversions']),
                           'direct_calls': len(entry['calls'])} for entry in observed]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--cli', type=Path, default=Path('target/debug/cargo-rstyle'))
    parser.add_argument('--driver', type=Path, default=Path('target/debug/rstyle-driver'))
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / 'selection.json').read_text())
    owner = json.loads((directory / selection['source_identity']['existing_selection']).read_text())
    root = Path(owner['package']['source_root'])
    oracle = load_oracle(directory)
    oracle.verify_sources(directory, selection, root)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    request_path = output / 'requests.json'
    request_path.write_text(json.dumps(requests()) + '\n')
    results = []
    for context in selection['build']['contexts']:
        destination = output / context['name']
        destination.mkdir()
        environment = dict(os.environ)
        for key in list(environment):
            if key.startswith(('RSTYLE_', 'CARGO_PROFILE_', 'CARGO_TARGET_')) or key in {
                    'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                    'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET'}:
                environment.pop(key, None)
        environment.update({
            'RSTYLE_DRIVER': str(args.driver.resolve()),
            'RSTYLE_DIAGNOSTIC_CAPTURE_ONLY': '1',
            'RSTYLE_DIAGNOSTIC_SHARED_SOURCES': '1',
            'RSTYLE_DIAGNOSTIC_FACTS_DIR': str(destination / 'snapshots'),
            'RSTYLE_DIAGNOSTIC_REFERENCE_INSTANCE_REQUESTS': str(request_path),
            'CARGO_TARGET_DIR': str(destination / 'target'),
            'RUSTUP_TOOLCHAIN': selection['build']['toolchain'],
        })
        command = [str(args.cli.resolve()), 'clones', '--manifest-path', str(root / 'Cargo.toml'),
                   '--offline', '--format', 'json', '--report-detail', 'compact',
                   '--extraction-profile', 'hir-data-v3', '--max-pairs', '0']
        if not context['default_features']:
            command.append('--no-default-features')
        if context['features']:
            command += ['--features', ','.join(context['features'])]
        with (destination / 'stdout.json').open('w') as stdout, (destination / 'stderr.log').open('w') as stderr:
            completed = subprocess.run(command, env=environment, stdout=stdout, stderr=stderr, timeout=240)
        result = dict(summarize(destination, completed, selection), context=context['name'])
        results.append(result)
        print(json.dumps(result), flush=True)
    oracle.verify_sources(directory, selection, root)
    (output / 'summary.json').write_text(json.dumps({'scope': 'owning diagnostics only',
        'unchanged_selected_source': True, 'contexts': results}, indent=2) + '\n')


if __name__ == '__main__':
    main()
