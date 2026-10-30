#!/usr/bin/env python3
"""Verify frozen source identities, optionally run the unchanged-library native oracle."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def verify_sources(selection, package):
    for relative, expected in selection['package']['file_sha256'].items():
        actual = hashlib.sha256((package / relative).read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f'source identity changed: {relative}')
    for item in selection['functions'] + selection['owners']:
        source = (package / item['file']).read_bytes()[item['span']['start']:item['span']['end']]
        if source.decode() != item['source'] or hashlib.sha256(source).hexdigest() != item['sha256']:
            raise ValueError(f'selected source changed: {item["name"]}')


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
    package = (args.package_root or Path(selection['package']['source_root'])).resolve()
    verify_sources(selection, package)
    if args.verify_only:
        print('Frozen source identities verified; no builds or execution.')
        return
    environment = dict(os.environ, RUSTUP_TOOLCHAIN=selection['build']['toolchain'])
    records = []
    with tempfile.TemporaryDirectory(prefix='rstyle-uncompressed-size-native-') as temporary:
        workspace = Path(temporary)
        (workspace / 'src').mkdir()
        (workspace / 'src/main.rs').write_text((directory / 'oracle.rs').read_text())
        for context in selection['build']['contexts']:
            manifest = ('[package]\nname="original_uncompressed_size"\nversion="0.0.0"\nedition="2021"\n'
                        '[workspace]\n[dependencies.lz4_flex]\n'
                        f'path={json.dumps(str(package))}\n'
                        f'default-features={str(context["default_features"]).lower()}\n'
                        f'features={json.dumps(context["features"])}\n')
            (workspace / 'Cargo.toml').write_text(manifest)
            command = [args.cargo, 'run', '--offline', '--quiet', '--manifest-path', str(workspace / 'Cargo.toml')]
            if args.target_dir:
                command += ['--target-dir', str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=environment, text=True, capture_output=True)
            if completed.returncode:
                raise RuntimeError(completed.stderr)
            records.append(dict(json.loads(completed.stdout), context=context['name']))
    result = {'identity': 'rstyle.original-uncompressed-size-native-oracle/1',
              'source_identities_checked': True, 'contexts': records,
              'limitation': 'Native runtime only; extraction, strict admission and matching are separate.'}
    rendered = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end='')


if __name__ == '__main__':
    main()
