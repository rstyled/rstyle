#!/usr/bin/env python3
"""Draft native unchanged-library prefilter oracle; cross-target capture is separate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def verify_sources(selection, package):
    for relative, expected in selection['package']['file_sha256'].items():
        assert hashlib.sha256((package / relative).read_bytes()).hexdigest() == expected, relative
    for item in selection['functions'] + selection['owners'] + selection['complete_impls'] + [selection['trait']]:
        original = (package / item['file']).read_bytes()[item['span']['start']:item['span']['end']]
        assert original.decode() == item['source'], item['name']
        assert hashlib.sha256(original).hexdigest() == item['sha256'], item['name']


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
        print('Frozen source hashes and spans verified; no builds or execution.')
        return
    environment = dict(os.environ, RUSTUP_TOOLCHAIN=selection['build']['toolchain'])
    records = []
    with tempfile.TemporaryDirectory(prefix='rstyle-prefilter-prefix-native-') as temporary:
        workspace = Path(temporary)
        (workspace / 'src').mkdir()
        (workspace / 'src/main.rs').write_text((directory / 'oracle.rs').read_text())
        for context in selection['build']['contexts']:
            enabled = context['name'] == 'default'
            manifest = ('[package]\nname="original_prefilter_prefix"\nversion="0.0.0"\nedition="2021"\n'
                        '[workspace]\n[dependencies.regex-automata]\n'
                        f'path={json.dumps(str(package))}\ndefault-features={str(enabled).lower()}\n')
            (workspace / 'Cargo.toml').write_text(manifest)
            command = [args.cargo, 'run', '--offline', '--quiet', '--manifest-path', str(workspace / 'Cargo.toml')]
            if args.target_dir:
                command += ['--target-dir', str(args.target_dir.resolve())]
            command += ['--', context['name']]
            result = subprocess.run(command, env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            if result.returncode:
                raise RuntimeError(result.stderr)
            record = json.loads(result.stdout)
            record.update(context=context['name'], concrete_prefix_execution=enabled)
            records.append(record)
    result = {'identity': 'rstyle.original-prefilter-prefix-native-oracle/1',
              'scope': 'unchanged public full-library native API; no-default has constructor checks only',
              'source_identities_checked': True, 'contexts': records}
    rendered = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end='')


if __name__ == '__main__':
    main()
