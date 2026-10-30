#!/usr/bin/env python3
"""Capture native original-owner scalar projections with retained Cargo source context.

Diagnostic only: no caller/artifact-binding or admission claim."""
import argparse
import importlib.util
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def flag(arguments, name):
    values = []
    for index, argument in enumerate(arguments):
        if argument == name and index + 1 < len(arguments):
            values.append(arguments[index + 1])
        elif argument.startswith(name + '='):
            values.append(argument[len(name) + 1:])
    if len(values) != 1:
        raise ValueError(f'expected one {name}: {values}')
    return values[0]


def codegen(arguments, name):
    values = []
    for index, argument in enumerate(arguments):
        value = argument[2:] if argument.startswith('-C') else ''
        if argument == '-C' and index + 1 < len(arguments):
            value = arguments[index + 1]
        if value.startswith(name + '='):
            values.append(value[len(name) + 1:])
    if len(values) != 1:
        raise ValueError(f'expected one -C{name}: {values}')
    return values[0]


def public_environment(environment):
    exact = {'CARGO_MANIFEST_DIR', 'CARGO_MANIFEST_PATH', 'OUT_DIR', 'OPT_LEVEL',
             'PROFILE', 'DEBUG', 'HOST', 'TARGET', 'NUM_JOBS', 'RUSTUP_TOOLCHAIN',
             'CARGO_ENCODED_RUSTFLAGS', 'RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'}
    prefixes = ('CARGO_PKG_', 'CARGO_FEATURE_', 'CARGO_CFG_')
    return {key: value for key, value in sorted(environment.items())
            if key in exact or key.startswith(prefixes)}


def wrapper(arguments):
    # Cargo supplies the real rustc executable, then its untouched arguments.
    original = subprocess.run(arguments)
    if original.returncode:
        return original.returncode
    try:
        name = flag(arguments[1:], '--crate-name')
    except ValueError:
        return 0  # rustc probe; not an owning library unit
    if name != 'num_traits' or '--crate-type' not in arguments and not any(
            item.startswith('--crate-type=') for item in arguments):
        return 0
    if 'lib' not in flag(arguments[1:], '--crate-type').split(','):
        return 0
    directory = Path(os.environ['RSTYLE_CLAMP_WRAPPER_OUTPUT'])
    directory.mkdir(parents=True, exist_ok=True)
    record = {'scope': 'diagnostic owner-local projection only', 'original_status': 0,
              'argv': arguments, 'cwd': os.getcwd(),
              'public_semantic_environment': public_environment(os.environ)}
    try:
        # Match the original frozen Cargo registry package, not just its crate name.
        expected = Path(os.environ['RSTYLE_CLAMP_SOURCE_ROOT']).resolve()
        if Path(os.environ['CARGO_MANIFEST_DIR']).resolve() != expected:
            raise ValueError('num_traits manifest directory differs from frozen original')
        out = Path(flag(arguments[1:], '--out-dir'))
        suffix = codegen(arguments[1:], 'extra-filename')
        metadata = out / f'libnum_traits{suffix}.rmeta'
        before = metadata.read_bytes()
        (directory / 'original.rmeta').write_bytes(before)
        record['metadata'] = {'path': str(metadata), 'bytes': len(before),
                              'native_header_hex': before[:40].hex()}
        # Retain all original environment privately, persist only the public subset.
        diagnostic = dict(os.environ)
        diagnostic.update({
            'RSTYLE_DIAGNOSTIC_CAPTURE_ONLY': '1',
            'RSTYLE_DIAGNOSTIC_SHARED_SOURCES': '1',
            'RSTYLE_DIAGNOSTIC_FACTS_DIR': str(directory / 'snapshots'),
            'RSTYLE_DIAGNOSTIC_INSTANCE_REQUESTS': os.environ['RSTYLE_CLAMP_REQUESTS'],
            'RSTYLE_HIR_FACTS_FILE': str(directory / 'not-published-facts.json'),
        })
        command = [os.environ['RSTYLE_CLAMP_DRIVER'],
                   '--rstyle-extraction-profile=hir-data-v3',
                   '--rstyle-selected-unit=' + Path(os.environ['RSTYLE_CLAMP_UNIT_CONTEXT']).read_text(),
                   *arguments]
        with (directory / 'driver.stdout').open('w') as stdout, (directory / 'driver.stderr').open('w') as stderr:
            projected = subprocess.run(command, env=diagnostic, stdout=stdout, stderr=stderr)
        record['diagnostic_status'] = projected.returncode
        record['metadata']['unchanged'] = metadata.read_bytes() == before
        if not record['metadata']['unchanged']:
            raise ValueError('diagnostic pass changed original emitted metadata')
        record['complete_publication_absent'] = not (directory / 'not-published-facts.json').exists()
    except Exception as error:
        record['error'] = str(error)
    (directory / 'wrapper.json').write_text(json.dumps(record, indent=2) + '\n')
    # Diagnostic failure is reported separately; original Cargo compilation remains valid.
    return 0


def load_oracle(directory):
    specification = importlib.util.spec_from_file_location('clamp_oracle', directory / 'oracle.py')
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def summarize(directory):
    wrapper_path = directory / 'wrapper.json'
    if not wrapper_path.exists():
        return {'error': 'owning num_traits library wrapper was not observed'}
    record = json.loads(wrapper_path.read_text())
    summary = {key: record[key] for key in ['original_status', 'diagnostic_status',
               'error', 'complete_publication_absent'] if key in record}
    summary['metadata_unchanged'] = record.get('metadata', {}).get('unchanged')
    snapshots = list((directory / 'snapshots').glob('*-scalar-instances.json'))
    rosters = list((directory / 'snapshots').glob('*-pre-helper-certification.json'))
    if len(snapshots) != 1 or len(rosters) != 1:
        summary['snapshot_error'] = {'instance_snapshots': len(snapshots), 'definition_rosters': len(rosters)}
        return summary
    inventory = json.loads(snapshots[0].read_text())
    summary['obligations_verified'] = inventory['obligations_verified']
    roster = json.loads(rosters[0].read_text())
    if roster.get('schema') not in ('rstyle.diagnostic.raw-facts/2', 'rstyle.diagnostic.raw-facts/3'):
        raise ValueError('expected complete shared-source diagnostic snapshot')
    facts = roster['facts']
    summary['definition_count'] = len(facts)
    selected = [item for item in facts if item['name'] in {'clamp', 'clamp_min', 'clamp_max'}]
    expected_context = json.loads((directory.parent / 'selected-unit.json').read_text())
    summary['selected_owner_contexts_complete'] = len(selected) == 3 and all(
        item['build']['frontend_options'].get('rstyle.nominal_selected_unit') == expected_context
        and item['build']['frontend_options'].get('rstyle.nominal_source_census', {}).get('value', {}).get('context') == expected_context
        for item in selected)
    entries = []
    for entry in inventory['requests']:
        comparisons = entry.get('comparisons', [])
        entries.append({'request': entry['request'], 'status': entry['status'],
                        'reason': entry.get('reason'), 'comparisons': len(comparisons),
                        'comparison_statuses': {status: sum(item['status'] == status for item in comparisons)
                                                for status in sorted({item['status'] for item in comparisons})},
                        'adjustments': len(entry.get('adjustments', []))})
    summary['requests'] = entries
    summary['request_count'] = len(entries)
    return summary


def selected_unit(cargo, workspace, environment, root):
    """Use Cargo's actual package identity; never reconstruct a registry ID."""
    completed = subprocess.run(
        [cargo, 'metadata', '--offline', '--format-version', '1',
         '--manifest-path', str(workspace / 'Cargo.toml')],
        env=environment, text=True, capture_output=True, check=True, timeout=60)
    metadata = json.loads(completed.stdout)
    packages = [item for item in metadata['packages']
                if item['name'] == 'num-traits' and item['version'] == '0.2.19'
                and Path(item['manifest_path']).resolve() == root / 'Cargo.toml'
                and (item['source'] or '').startswith('registry+')]
    if len(packages) != 1:
        raise ValueError('selected unit lacks one exact original registry package')
    package = packages[0]
    targets = [item for item in package['targets'] if 'lib' in item['kind']]
    if len(targets) != 1 or targets[0]['name'] != 'num_traits':
        raise ValueError('selected unit lacks one original num_traits library')
    manifest_path = Path(package['manifest_path']).resolve()
    manifest = manifest_path.read_bytes()
    # Required PackageContext source identity; no additional hashes or lookup keys.
    return {'package': {'identity': 'rstyle.rust.cargo-package-capture/1',
            'package_id': package['id'], 'name': package['name'],
            'version': package['version'], 'manifest_path': str(manifest_path),
            'manifest_bytes': manifest.decode('utf-8'),
            'manifest_digest': 'sha256:' + hashlib.sha256(manifest).hexdigest()},
            'root_source': str(Path(targets[0]['src_path']).resolve()),
            'crate_name': targets[0]['name']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--driver', type=Path, default=Path('/tmp/rstyle-scalar-projection-bin/rstyle-driver'))
    parser.add_argument('--cargo', default='/home/dev/.cargo/bin/cargo')
    args = parser.parse_args()
    directory = args.directory.resolve()
    selection = json.loads((directory / 'selection.json').read_text())
    root = Path(selection['package']['source_root']).resolve()
    oracle = load_oracle(directory)
    oracle.verify_sources(selection, root)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    requests = output / 'requests.json'
    requests.write_text(json.dumps({'identity': 'rstyle.diagnostic.scalar-instance-requests/1',
                                   'crate_name': 'num_traits',
                                   'requests': [{'function': item['name'], 'arguments': [ty]}
                                                for item in selection['functions'] for ty in ['i32', 'u32']]}, indent=2) + '\n')
    records = []
    for context in selection['build']['contexts']:
        workspace = output / context['name']
        (workspace / 'src').mkdir(parents=True)
        (workspace / 'src/main.rs').write_text((directory / 'oracle.rs').read_text())
        (workspace / 'Cargo.toml').write_text(oracle.manifest(context))
        environment = dict(os.environ, RUSTUP_TOOLCHAIN=selection['build']['toolchain'])
        for key in list(environment):
            if key.startswith(('CARGO_PROFILE_', 'CARGO_TARGET_', 'RSTYLE_')) or key in {
                    'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTDOC',
                    'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET'}:
                environment.pop(key, None)
        features = oracle.resolved_original(args.cargo, workspace, environment, root)
        if sorted(features) != sorted(context['effective_features']):
            raise ValueError(f'unexpected resolved features: {features}')
        unit_context = workspace / 'selected-unit.json'
        unit_context.write_text(json.dumps(selected_unit(args.cargo, workspace, environment, root), separators=(',', ':')))
        diagnostics = workspace / 'owner'
        environment.update({
            'RUSTC_WRAPPER': str(Path(__file__).resolve()),
            'RUSTC_WORKSPACE_WRAPPER': '',
            'CARGO_TARGET_DIR': str(workspace / 'target'), 'CARGO_INCREMENTAL': '0',
            'RSTYLE_CLAMP_WRAPPER_OUTPUT': str(diagnostics),
            'RSTYLE_CLAMP_SOURCE_ROOT': str(root),
            'RSTYLE_CLAMP_REQUESTS': str(requests),
            'RSTYLE_CLAMP_DRIVER': str(args.driver.resolve()),
            'RSTYLE_CLAMP_UNIT_CONTEXT': str(unit_context),
        })
        command = [args.cargo, 'check', '--offline', '--manifest-path', str(workspace / 'Cargo.toml')]
        with (workspace / 'cargo.stdout').open('w') as stdout, (workspace / 'cargo.stderr').open('w') as stderr:
            compiled = subprocess.run(command, env=environment, stdout=stdout, stderr=stderr)
        oracle.verify_sources(selection, root)
        records.append(dict(summarize(diagnostics), context=context['name'],
                            cargo_status=compiled.returncode, effective_features=features))
        record = records[-1]
        print(json.dumps({key: record.get(key) for key in ['context', 'cargo_status',
            'diagnostic_status', 'metadata_unchanged', 'request_count', 'error', 'snapshot_error']}), flush=True)
    result = {'scope': 'diagnostic owner-local only; no caller/artifact-binding or admission claim',
              'unchanged_exact_selected_source': True, 'contexts': records}
    (output / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    if not all(record.get('cargo_status') == 0 and record.get('diagnostic_status') == 0
               and record.get('metadata_unchanged') is True
               and record.get('complete_publication_absent') is True
               and record.get('selected_owner_contexts_complete') is True
               and record.get('request_count') == 6
               and all(entry['status'] == 'projected' for entry in record.get('requests', []))
               and 'error' not in record and 'snapshot_error' not in record
               for record in records):
        raise SystemExit('capture incomplete; see summary.json')


if __name__ == '__main__':
    if os.environ.get('RSTYLE_CLAMP_WRAPPER_OUTPUT'):
        sys.exit(wrapper(sys.argv[1:]))
    main()
