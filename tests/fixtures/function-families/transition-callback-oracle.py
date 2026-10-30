#!/usr/bin/env python3
"""Check full unchanged package against independently specified byte membership."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--native', help='executable linked to the unchanged full package')
    args = parser.parse_args()
    selection = json.loads(Path(__file__).with_name('transition-callback-selection.json').read_text())
    root = args.source_root or Path(selection['package']['source_root'])
    for name, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((root / name).read_bytes()).hexdigest() == digest, name
    for function in selection['functions']:
        source = (root / function['file']).read_bytes()
        span = function['span']
        assert hashlib.sha256(source[span['start']:span['end']]).hexdigest() == function['sha256']
    if args.native:
        process = subprocess.Popen([args.native], stdout=subprocess.PIPE, text=True)
        for start in range(256):
            for end in range(256):
                # Integer set membership specifies semantics independently of
                # the Rust implementation's comparison/conjunction expression.
                members = set(range(start, end + 1))
                mask = bytes(sum(1 << bit for bit in range(8)
                                 if 8 * chunk + bit in members) for chunk in range(32)).hex()
                expected = f'{start} {end} {mask} {mask} {mask}\n'
                assert process.stdout.readline() == expected, (start, end)
        assert process.stdout.read() == '', 'unexpected extra rows'
        assert process.wait(timeout=60) == 0
    print(json.dumps({'source_identity': 'verified', 'native': bool(args.native),
                      'triples': 256 ** 3, 'next_identities': 2,
                      'families': ['matches_byte', 'matches', 'matches_unit']}))


if __name__ == '__main__':
    main()
