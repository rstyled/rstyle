#!/usr/bin/env python3
"""Independent endpoint expectations for the unchanged public Range getter chain.

This script validates frozen source identities before running an externally built
native binary. It neither builds code nor substitutes copied-source witnesses.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent


def cases(width):
    maximum = (1 << width) - 1
    points = set(range(17))
    for boundary in (255, 256, 65535, 65536, 1 << (width - 1), maximum):
        points.update(n for n in range(boundary - 2, boundary + 3)
                      if 0 <= n <= maximum)
    rows = [('S', start, end, pattern) for start in sorted(points)
            for end in sorted(points) for pattern in (0, 1, (1 << 31) - 2)]
    rows.extend(('I', start, end, 32) for end in range(33)
                for start in range(end + 2))
    rows.extend(('P', start, end, 0)
                for start, end in ((1, 0), (maximum, 0), (maximum, maximum - 1)))
    return rows


def expected(row):
    kind, start, end, _ = row
    if kind == 'I':
        return f'I 0 32 {start} {end}'
    if kind == 'P':
        return 'P rejected'
    matched = f'{start} {end}' if start <= end else 'none none'
    return f'S {start} {end} {start} {end} {start} {end} {matched}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--native', type=Path)
    parser.add_argument('--pointer-width', type=int, choices=(32, 64), default=64)
    args = parser.parse_args()
    selection = json.loads((ROOT.parent / 'range-getters-selection.json').read_text())
    source = args.source_root or Path(selection['package']['source_root'])
    for name, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source / name).read_bytes()).hexdigest() == digest, name
    for function in selection['functions'] + selection['source_owners']:
        raw = (source / function['file']).read_bytes()
        span = function['span']
        assert hashlib.sha256(raw[span['start']:span['end']]).hexdigest() == function['sha256'], function['name']
    rows = cases(args.pointer_width)
    request = ''.join(' '.join(map(str, row)) + '\n' for row in rows)
    digest = hashlib.sha256(request.encode()).hexdigest()
    frozen = selection['sample'][str(args.pointer_width)]
    assert len(rows) == frozen['cases'] and digest == frozen['request_sha256']
    if args.native:
        run = subprocess.run([str(args.native.resolve())], input=request, text=True,
                             capture_output=True, check=True, timeout=60)
        lines = run.stdout.splitlines()
        assert lines[0] == f'width {args.pointer_width}', lines[:1]
        assert len(lines) == len(rows) + 1, len(lines)
        for row, actual in zip(rows, lines[1:]):
            assert actual == expected(row), (row, actual, expected(row))
    print(json.dumps({'cases': len(rows), 'pointer_width': args.pointer_width,
                      'native_checked': bool(args.native), 'request_sha256': digest,
                      'span_cases': sum(row[0] == 'S' for row in rows),
                      'input_cases': sum(row[0] == 'I' for row in rows),
                      'match_constructor_panic_cases': 3}, sort_keys=True))


if __name__ == '__main__':
    main()
