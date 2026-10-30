#!/usr/bin/env python3
"""Independent mathematical lengths for unchanged original Span/Match methods.

Selection and expected classifications precede extraction. Python subtraction is
unbounded; Match construction is tested only inside its public ordered domain.
The native binary must link the full unchanged declared library build context.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent


def cases(width):
    maximum = (1 << width) - 1
    points = set(range(33))
    for boundary in [maximum, maximum // 2, 1 << (width - 1), 255, 256, 65535, 65536]:
        points.update(n for n in range(boundary - 2, boundary + 3) if 0 <= n <= maximum)
    return [(start, end, pattern) for start in sorted(points)
            for end in sorted(points) for pattern in (0, 1, (1 << 31) - 2)]


def expected(row):
    start, end, _ = row
    size = max(end - start, 0)
    empty = int(end <= start)
    if start <= end:
        return f'{size} {empty} {size} {empty} {start} {end}'
    return f'{size} {empty} none none none none'


def verify_source(selection, source):
    for relative, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source / relative).read_bytes()).hexdigest() == digest, relative
    for function in selection['functions']:
        original = (source / function['file']).read_bytes()
        span = function['span']
        observed = hashlib.sha256(original[span['start']:span['end']]).hexdigest()
        assert observed == function['sha256'], function['name']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--native')
    parser.add_argument('--pointer-width', type=int, choices=(32, 64), default=64)
    args = parser.parse_args()
    selection = json.loads((ROOT / 'span-length-selection.json').read_text())
    source = args.source_root or Path(selection['package']['source_root'])
    verify_source(selection, source)
    rows = cases(args.pointer_width)
    request = ''.join(f'{start} {end} {pattern}\n' for start, end, pattern in rows)
    digest = hashlib.sha256(request.encode()).hexdigest()
    frozen = selection['sample'][str(args.pointer_width)]
    assert len(rows) == frozen['cases'] and digest == frozen['request_sha256']
    if args.native:
        run = subprocess.run([args.native], input=request, text=True,
                             capture_output=True, check=True, timeout=60)
        lines = run.stdout.splitlines()
        assert lines[0] == f'width {args.pointer_width}', lines[:1]
        assert len(lines) == len(rows) + 1, len(lines)
        for row, actual in zip(rows, lines[1:]):
            assert actual == expected(row), (row, actual, expected(row))
    print(json.dumps({'cases': len(rows), 'pointer_width': args.pointer_width,
                      'native_checked': bool(args.native), 'request_sha256': digest,
                      'ordered_match_cases': sum(start <= end for start, end, _ in rows),
                      'inverted_span_cases': sum(start > end for start, end, _ in rows)},
                     sort_keys=True))


if __name__ == '__main__':
    main()
