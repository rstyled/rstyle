#!/usr/bin/env python3
"""Verify source selection; optionally run unchanged registry num-traits natively."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def verify_sources(selection, root):
    for item in selection['functions']:
        source = (root / item['file']).read_bytes()[item['span']['start']:item['span']['end']]
        if source.decode() != item['source']:
            raise ValueError(f'selected original source changed: {item["name"]}')
    for relative, expected in selection['package']['exact_files'].items():
        if (root / relative).read_text() != expected:
            raise ValueError(f'package context source changed: {relative}')


def manifest(context):
    debug = str(context['debug_assertions']).lower()
    defaults = str(context['default_features']).lower()
    return ('[package]\nname="original_clamp_instances"\nversion="0.0.0"\nedition="2024"\n'
            '[workspace]\n[dependencies.num-traits]\nversion="=0.2.19"\n'
            f'default-features={defaults}\n'
            '[profile.dev]\npanic="unwind"\noverflow-checks=true\nopt-level=0\n'
            f'debug-assertions={debug}\n')


def resolved_original(cargo, workspace, environment, root):
    completed = subprocess.run(
        [cargo, 'metadata', '--offline', '--format-version', '1',
         '--manifest-path', str(workspace / 'Cargo.toml')],
        env=environment, text=True, capture_output=True, check=True, timeout=60)
    metadata = json.loads(completed.stdout)
    matches = [item for item in metadata['packages']
               if item['name'] == 'num-traits' and item['version'] == '0.2.19']
    if len(matches) != 1 or not (matches[0]['source'] or '').startswith('registry+'):
        raise ValueError('expected one original num-traits=0.2.19 registry package')
    if Path(matches[0]['manifest_path']).resolve() != root / 'Cargo.toml':
        raise ValueError('Cargo selected a different registry source root')
    node = next(item for item in metadata['resolve']['nodes'] if item['id'] == matches[0]['id'])
    return node['features']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package-root', type=Path)
    parser.add_argument('--cargo', default='cargo')
    parser.add_argument('--target-dir', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / 'selection.json').read_text())
    root = (args.package_root or Path(selection['package']['source_root'])).resolve()
    verify_sources(selection, root)
    if args.verify_only:
        print('Exact selected sources verified; no builds or execution.')
        return
    environment = dict(os.environ, RUSTUP_TOOLCHAIN=selection['build']['toolchain'])
    for key in list(environment):
        if key.startswith('CARGO_PROFILE_') or key.startswith('CARGO_TARGET_') or key in {
            'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
            'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET', 'RUSTC', 'RUSTDOC',
        }:
            environment.pop(key, None)
    records = []
    with tempfile.TemporaryDirectory(prefix='rstyle-clamp-instances-') as temporary:
        workspace = Path(temporary)
        (workspace / 'src').mkdir()
        (workspace / 'src/main.rs').write_text((directory / 'oracle.rs').read_text())
        for context in selection['build']['contexts']:
            (workspace / 'Cargo.toml').write_text(manifest(context))
            features = resolved_original(args.cargo, workspace, environment, root)
            if sorted(features) != sorted(context['effective_features']):
                raise ValueError(f'unexpected resolved features: {features}')
            command = [args.cargo, 'run', '--offline', '--quiet',
                       '--manifest-path', str(workspace / 'Cargo.toml')]
            if args.target_dir:
                command += ['--target-dir', str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=environment, text=True,
                                       capture_output=True, check=True, timeout=180)
            record = json.loads(completed.stdout)
            if record['debug_assertions'] != context['debug_assertions']:
                raise ValueError('runtime debug mode disagrees with selection')
            for key, expected in context['expected_oracle'].items():
                if record.get(key) != expected:
                    raise ValueError(f'unexpected oracle {key}: {record.get(key)} != {expected}')
            records.append(dict(record, context=context['name'], effective_features=features))
    result = {
        'identity': 'rstyle.original-clamp-instances-native-oracle/1',
        'source_selection_verified': True, 'contexts': records,
        'limitation': 'Native runtime only; no extraction, admission or matching claim.',
    }
    rendered = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end='')


if __name__ == '__main__':
    main()
