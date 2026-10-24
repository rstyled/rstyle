#!/usr/bin/env python3
"""Check frozen sources; optionally build and run the unchanged-library native oracle."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def source_tree(root):
    files = sorted(set([root / 'Cargo.toml', root / 'Cargo.toml.orig',
                        *root.glob('src/**/*.rs')]))
    entries = [(str(path.relative_to(root)), sha256(path.read_bytes()))
               for path in files if path.is_file()]
    return sha256(json.dumps(entries, separators=(',', ':')).encode())


def verify_sources(selection, roots):
    for package in selection['packages']:
        root = roots[package['name']]
        if source_tree(root) != package['source_tree_sha256']:
            raise ValueError(f'package source tree changed: {package["name"]}')
        for relative, expected in package['file_sha256'].items():
            if sha256((root / relative).read_bytes()) != expected:
                raise ValueError(f'file source changed: {package["name"]}/{relative}')
    for item in selection['functions'] + selection['owners']:
        source = (roots[item['package']] / item['file']).read_bytes()[
            item['span']['start']:item['span']['end']]
        if source.decode() != item['source'] or sha256(source) != item['sha256']:
            raise ValueError(f'selected source changed: {item["name"]}')


def manifest(context):
    return ('[package]\nname="original_calendar_helpers"\nversion="0.0.0"\nedition="2024"\n'
            '[workspace]\n[dependencies.time]\nversion="=0.3.47"\n'
            f'default-features={str(context["default_features"]).lower()}\n'
            f'features={json.dumps(context["features"])}\n'
            '[dependencies.time-core]\nversion="=0.1.8"\n')


def verify_resolution(args, workspace, environment, roots):
    completed = subprocess.run(
        [args.cargo, 'metadata', '--offline', '--format-version', '1',
         '--manifest-path', str(workspace / 'Cargo.toml')],
        env=environment, text=True, capture_output=True)
    if completed.returncode:
        raise RuntimeError(completed.stderr)
    packages = json.loads(completed.stdout)['packages']
    for name, version in [('time', '0.3.47'), ('time-core', '0.1.8')]:
        selected = [package for package in packages
                    if package['name'] == name and package['version'] == version]
        if len(selected) != 1 or not selected[0]['source'].startswith('registry+'):
            raise ValueError(f'expected one original registry package: {name}={version}')
        if Path(selected[0]['manifest_path']).resolve() != roots[name] / 'Cargo.toml':
            raise ValueError(f'Cargo selected a different source root: {name}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--time-root', type=Path)
    parser.add_argument('--time-core-root', type=Path)
    parser.add_argument('--cargo', default='cargo')
    parser.add_argument('--target-dir', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / 'selection.json').read_text())
    roots = {package['name']: Path(package['source_root']).resolve()
             for package in selection['packages']}
    for name, override in [('time', args.time_root), ('time-core', args.time_core_root)]:
        if override:
            roots[name] = override.resolve()
    verify_sources(selection, roots)
    if args.verify_only:
        print('Frozen source identities verified; no builds or runtime execution.')
        return
    environment = dict(os.environ, RUSTUP_TOOLCHAIN=selection['build']['toolchain'])
    for key in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET']:
        environment.pop(key, None)
    records = []
    with tempfile.TemporaryDirectory(prefix='rstyle-calendar-helpers-native-') as temporary:
        workspace = Path(temporary)
        (workspace / 'src').mkdir()
        (workspace / 'src/main.rs').write_text((directory / 'oracle.rs').read_text())
        for context in selection['build']['contexts']:
            (workspace / 'Cargo.toml').write_text(manifest(context))
            verify_resolution(args, workspace, environment, roots)
            command = [args.cargo, 'run', '--offline', '--quiet',
                       '--manifest-path', str(workspace / 'Cargo.toml')]
            if args.target_dir:
                command += ['--target-dir', str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=environment, text=True, capture_output=True)
            if completed.returncode:
                raise RuntimeError(completed.stderr)
            records.append(dict(json.loads(completed.stdout), context=context['name']))
    result = {'identity': 'rstyle.original-calendar-helpers-native-oracle/1',
              'source_identities_checked': True, 'contexts': records,
              'limitation': 'Native runtime only; capture, strict admission and matching are separate.'}
    rendered = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end='')


if __name__ == '__main__':
    main()
