#!/usr/bin/env python3
"""Capture bounded original Match instances; diagnostic evidence grants no admission."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify_sources(directory, selection):
    spec = importlib.util.spec_from_file_location('match_oracle', directory / 'oracle.py')
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    oracle.verify_sources(selection, Path(selection['package']['source_root']))


def requests():
    return {'identity': 'rstyle.diagnostic.product-instance-requests/1',
            'crate_name': 'regex_automata', 'requests': [
                {'function': 'util::search::Match::new',
                 'arguments': ['Span', 'Range<usize>']}]}


def original_source(facts, expected):
    require(facts['source']['bytes'] == expected['source']
            and facts['fn_span'] == expected['span'],
            'observed source/span differs from independent source selection')
    require(isinstance(facts['source_scan_complete'], bool)
            and isinstance(facts['source_diagnostics'], list),
            'original source scan metadata is malformed')


def definition(value, path, local):
    require(value['path'] == path and value['local'] is local
            and value['crate'] == ('regex_automata' if local else 'core')
            and value['definition_identity'] and value['stable_crate_id'],
            'unexpected selected definition: ' + path)
    if not local:
        require(bool(value.get('crate_hash')), 'foreign compiler identity missing crate hash')


def check_instance(instance, selection):
    require(instance['status'] == 'observed', 'selected original instance unavailable')
    argument = instance['argument']
    require(argument in ['Span', 'Range<usize>'], 'unexpected substitution')
    owners = instance['owners']
    require(len(owners) == 2 and not owners[0]['parameters']
            and len(owners[1]['parameters']) == 1
            and owners[1]['parent'] == owners[0]['definition'], 'original owner chain differs')
    parameter = owners[1]['parameters'][0]
    require((parameter['index'], parameter['name']) == (0, 'S'), 'original S binder missing')
    require(instance['original_signature']['inputs'][1]
            == {'kind': 'parameter', 'index': 0, 'name': 'S'}, 'original S input missing')
    require(instance['projected_inputs'][1] == instance['arguments'], 'S projection differs')
    require(len(instance['conversions']) == 1 and len(instance['nodes']) == 16,
            'original node/conversion roster differs')
    conversion = instance['conversions'][0]
    require(conversion['identity'] == 'rstyle.rust.product-into-from-observation/1'
            and conversion['receiver_type'] == instance['arguments']
            and not conversion['adjustments'], 'conversion receiver evidence differs')
    input_type = conversion['receiver_type'][0]['identity']
    span_type = conversion['result_type'][0]['identity']
    require(span_type['kind'] == 'adt' and span_type['arguments'] == []
            and span_type['definition']['path'] == 'util::search::Span'
            and span_type['definition']['local'] is True, 'actual Span identity missing')
    if argument == 'Span':
        require(input_type == span_type, 'Span substitution differs')
    else:
        require(input_type['kind'] == 'adt'
                and input_type['definition']['path'] == 'core::ops::Range'
                and input_type['definition']['crate'] == 'core'
                and input_type['definition']['local'] is False
                and input_type['arguments'] == [{'kind': 'builtin', 'name': 'usize'}],
                'Range<usize> structural substitution differs')
    for key, trait in [('into', 'Into'), ('from', 'From')]:
        selected = conversion[key]
        definition(selected['requested'], f'core::convert::{trait}::{key}', False)
        definition(selected['requested_trait'], f'core::convert::{trait}', False)
        require(selected['requested_trait'] == selected['selected_trait']
                and selected['requested_arguments'] == selected['substituted_impl_trait_arguments']
                and selected['requested_signature'] == selected['selected_signature']
                and selected['requested_signature']['bound_variables'] == 0,
                'selected conversion trait/substitution/signature differs')
        expected = ([input_type, span_type] if key == 'into' else [span_type, input_type])
        require([value['identity'] for value in selected['requested_arguments']] == expected,
                'conversion type argument order differs')
    definition(conversion['into']['selected'], '<T as core::convert::Into<U>>::into', False)
    definition(conversion['into']['selected_impl'], '<T as core::convert::Into<U>>', False)
    body = instance['selected_local_from']
    if argument == 'Span':
        definition(conversion['from']['selected'], '<T as core::convert::From<T>>::from', False)
        definition(conversion['from']['selected_impl'], '<T as core::convert::From<T>>', False)
        require(body is None, 'reflexive core conversion unexpectedly has local source')
    else:
        owner = '<util::search::Span as core::convert::From<core::ops::Range<usize>>>'
        definition(conversion['from']['selected'], owner + '::from', True)
        definition(conversion['from']['selected_impl'], owner, True)
        obligation = next(item for item in selection['source_obligations']
                          if item['name'] == 'From<Range<usize>> for Span')
        # Select the method only from the independently frozen complete impl.
        text = obligation['source']
        offset = text.index('fn from(')
        source = text[offset:text.rindex('\n}')]
        start = obligation['span']['start'] + len(text[:offset].encode())
        original_source(body, {'source': source, 'span': {'start': start,
                        'end': start + len(source.encode())}})
        require(body['definition'] == conversion['from']['selected'], 'local body identity differs')
    return {'argument': argument, 'nodes': len(instance['nodes']), 'conversions': 1,
            'local_from_body': body is not None,
            'local_from_source_scan_complete': body['source_scan_complete'] if body else None,
            'local_from_source_diagnostics': body['source_diagnostics'] if body else None}


def summarize(directory, selection, context, exit_code, expected_target=None):
    require(exit_code == 2, 'capture-only invocation must exit two')
    report = json.loads((directory / 'stdout.json').read_text())
    require(report['capture_complete'] is False and not report['functions'],
            'capture-only unexpectedly published functions')
    files = list((directory / 'snapshots').glob('*-product-instances.json'))
    require(len(files) == 1, 'expected exactly one owning product inventory')
    inventory = json.loads(files[0].read_text())
    require(inventory['identity'] == 'rstyle.diagnostic.product-instance-bodies/1'
            and inventory['admission_authority'] is False and len(inventory['requests']) == 1,
            'unexpected diagnostic inventory authority/roster')
    entry = inventory['requests'][0]
    require(entry['request'] == requests()['requests'][0] and entry['status'] == 'observed',
            'original request unavailable')
    original_source(entry['original'], selection['functions'][0])
    build = entry['original']['build']
    require(expected_target is None or build['frontend_options']['target'] == expected_target,
            'captured target differs from explicit selection')
    require(build['rustc_commit']['value'] == selection['build_selection']['rustc_commit']
            and build['frontend_options']['target'] in selection['build_selection']['targets']
            and build['target_triple']['value'] == {
                'aarch64-apple-darwin': 'arm64-apple-macosx',
            }.get(build['frontend_options']['target'], build['frontend_options']['target'])
            and build['features'] == context['expected_feature_closure'], 'build context differs')
    instances = entry['instances']
    require([item['argument'] for item in instances] == ['Span', 'Range<usize>'],
            'selected instance roster differs')
    observations = [check_instance(item, selection) for item in instances]
    raw = list((directory / 'snapshots').glob('*-pre-helper-certification.json'))
    require(len(raw) == 1, 'missing complete original raw roster')
    roster = json.loads(raw[0].read_text())
    count = 2582 if context['default_features'] else 590
    require(roster.get('schema') in ('rstyle.diagnostic.raw-facts/2', 'rstyle.diagnostic.raw-facts/3')
            and len(roster['facts']) == count, 'complete raw definition roster differs')
    return {'context': context['name'], 'target': build['frontend_options']['target'],
            'definition_count': count, 'instances': observations,
            'admission_authority': False, 'capture_complete': False,
            'unchanged_selected_source': True,
            'original_source_scan_complete': entry['original']['source_scan_complete'],
            'original_source_diagnostics': entry['original']['source_diagnostics'],
            'source_closure_certified': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--context', choices=['no-default', 'default'], default='no-default')
    parser.add_argument('--target', help='capture an explicitly selected target; no native execution claim')
    parser.add_argument('--summarize', action='store_true', help='validate retained snapshot; do not run Cargo')
    parser.add_argument('--exit-code', type=int, default=2, help='recorded invocation exit code for --summarize')
    parser.add_argument('--cli', type=Path, default=Path('target/debug/cargo-rstyle'))
    parser.add_argument('--driver', type=Path, default=Path('target/debug/rstyle-driver'))
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / 'selection.json').read_text())
    require(args.target is None or args.target in selection['build_selection']['targets'],
            'target is outside the frozen selection')
    context = next(item for item in selection['build_selection']['contexts'] if item['name'] == args.context)
    verify_sources(directory, selection)
    output = args.output.resolve()
    exit_code = args.exit_code
    if not args.summarize:
        output.mkdir(parents=True, exist_ok=False)
        request_path = output / 'requests.json'
        request_path.write_text(json.dumps(requests()) + '\n')
        environment = dict(os.environ)
        for key in list(environment):
            if key.startswith(('RSTYLE_', 'CARGO_PROFILE_', 'CARGO_TARGET_')) or key in {
                    'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                    'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET'}:
                environment.pop(key, None)
        environment.update({
            'RSTYLE_DRIVER': str(args.driver.resolve()),
            'RSTYLE_DIAGNOSTIC_CAPTURE_ONLY': '1', 'RSTYLE_DIAGNOSTIC_SHARED_SOURCES': '1',
            'RSTYLE_DIAGNOSTIC_FACTS_DIR': str(output / 'snapshots'),
            'RSTYLE_DIAGNOSTIC_PRODUCT_INSTANCE_REQUESTS': str(request_path),
            'CARGO_TARGET_DIR': str(output / 'target'),
            'RUSTUP_TOOLCHAIN': selection['build_selection']['toolchain'],
        })
        command = [str(args.cli.resolve()), 'clones', '--manifest-path',
                   str(Path(selection['package']['source_root']) / 'Cargo.toml'),
                   '--offline', '--format', 'json', '--report-detail', 'compact',
                   '--extraction-profile', 'hir-data-v3', '--max-pairs', '0']
        if not context['default_features']:
            command.append('--no-default-features')
        if args.target:
            command.extend(['--target', args.target])
        with (output / 'stdout.json').open('w') as stdout, (output / 'stderr.log').open('w') as stderr:
            completed = subprocess.run(command, env=environment, stdout=stdout, stderr=stderr, timeout=240)
        exit_code = completed.returncode
    verify_sources(directory, selection)
    result = summarize(output, selection, context, exit_code, args.target)
    print(json.dumps(result))
    if not args.summarize:
        (output / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
