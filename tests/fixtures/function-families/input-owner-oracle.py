#!/usr/bin/env python3
"""Independent stored-bool expectations for unchanged Input::get_earliest.

Native input: haystack hex (or -), start, end, anchored mode, earliest0/1.
Output: default/configured/neighbor-changed/toggled/restored bools.
Constructors/setters create states only; their extraction is not acceptance scope.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
HAYSTACKS = ['', '00', 'ff', '616263', 'e29883', '0080ff0a0d7f0102']
ANCHORS = ['n', 'y', 'p0', 'p1', 'p17', 'pmax']


def requests():
    for spelling in HAYSTACKS:
        length = len(bytes.fromhex(spelling))
        for end in range(length + 1):
            for start in range(end + 2):
                for anchor in ANCHORS:
                    for earliest in (0, 1):
                        yield f'{spelling or "-"} {start} {end} {anchor} {earliest}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--native')
    parser.add_argument('--getters', action='store_true')
    args = parser.parse_args()
    selection = json.loads((ROOT / 'input-owner-selection.json').read_text())
    source = args.source_root or Path(selection['package']['source_root'])
    for relative, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source / relative).read_bytes()).hexdigest() == digest, relative
    assert HAYSTACKS == selection['sample']['haystacks_hex']
    assert ANCHORS == selection['sample']['anchors']
    rows = list(requests())
    request = '\n'.join(rows) + '\n'
    assert hashlib.sha256(request.encode()).hexdigest() == selection['sample']['request_sha256']
    if args.native:
        command = [args.native] + (['--getters'] if args.getters else [])
        run = subprocess.run(command, input=request, text=True, capture_output=True,
                             check=True, timeout=60)
        lines = run.stdout.splitlines()
        assert len(lines) == len(rows)
        for row, result in zip(rows, lines):
            earliest = int(row.split()[-1])
            expected = [0, earliest, earliest, 1 - earliest, earliest]
            if args.getters:
                start, end = map(int, row.split()[1:3])
                expected += [start, end, start, end, int(start > end)]
            assert [int(value) for value in result.split()] == expected, (row, result, expected)
    print(json.dumps({'states': len(rows), 'observations': len(rows) * (10 if args.getters else 5),
                      'native_checked': bool(args.native),
                      'request_sha256': selection['sample']['request_sha256']}, sort_keys=True))


if __name__ == '__main__':
    main()
