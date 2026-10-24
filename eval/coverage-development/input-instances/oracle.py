#!/usr/bin/env python3
"""Verify existing source pins and run concrete constructors from the original crate."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def verify_sources(directory, selection, source_root):
    # Preserve the existing source identity contract; do not add transport hashing.
    subprocess.run([sys.executable, str(directory.parent / 'input-owner-oracle.py'),
                    '--source-root', str(source_root)], check=True, capture_output=True)
    for function in selection['functions']:
        span = function['span']
        actual = (source_root / function['file']).read_bytes()[span['start']:span['end']]
        if actual != function['source'].encode():
            raise ValueError(f'original function changed: {function["name"]}')


def manifest(context):
    return ('[package]\nname="original_input_instances"\nversion="0.0.0"\n'
            'edition="2021"\n[workspace]\n[dependencies.regex-automata]\n'
            'version="=0.4.18"\n'
            f'default-features={str(context["default_features"]).lower()}\n'
            f'features={json.dumps(context["features"])}\n')


def verify_resolution(workspace, environment, source_root):
    result = subprocess.run(['cargo', 'metadata', '--offline', '--format-version', '1',
                             '--manifest-path', str(workspace / 'Cargo.toml')],
                            env=environment, text=True, capture_output=True,
                            check=True, timeout=60)
    packages = json.loads(result.stdout)['packages']
    selected = [package for package in packages
                if package['name'] == 'regex-automata' and package['version'] == '0.4.18']
    if (len(selected) != 1 or not selected[0]['source'].startswith('registry+')
            or Path(selected[0]['manifest_path']).resolve() != source_root / 'Cargo.toml'):
        raise ValueError('Cargo selected a different original package')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--target-dir', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / 'selection.json').read_text())
    inherited = json.loads((directory / selection['source_identity']['existing_selection']).read_text())
    source_root = (args.source_root or Path(inherited['package']['source_root'])).resolve()
    verify_sources(directory, selection, source_root)
    if args.verify_only:
        print('Existing source pins and both selected original bodies verified.')
        return
    environment = dict(os.environ, RUSTUP_TOOLCHAIN=selection['build']['toolchain'])
    for key in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET']:
        environment.pop(key, None)
    records = []
    with tempfile.TemporaryDirectory(prefix='rstyle-input-instances-native-') as temporary:
        workspace = Path(temporary)
        (workspace / 'src').mkdir()
        (workspace / 'src/main.rs').write_bytes((directory / 'oracle.rs').read_bytes())
        for context in selection['build']['contexts']:
            (workspace / 'Cargo.toml').write_text(manifest(context))
            verify_resolution(workspace, environment, source_root)
            command = ['cargo', 'run', '--offline', '--quiet',
                       '--manifest-path', str(workspace / 'Cargo.toml')]
            if args.target_dir:
                command += ['--target-dir', str(args.target_dir.resolve())]
            result = subprocess.run(command, env=environment, text=True,
                                    capture_output=True, check=True, timeout=180)
            records.append(dict(json.loads(result.stdout), context=context['name']))
    report = {'identity': 'rstyle.original-input-instances-native-oracle/1',
              'source_identities_checked': True, 'contexts': records,
              'limitation': selection['oracle']['limitations']}
    rendered = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end='')


if __name__ == '__main__':
    main()
