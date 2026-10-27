#!/usr/bin/env python3
"""Independent unbounded arithmetic for original base64::encoded_len.

The native executable links the complete unchanged package, in either declared
feature context. Python's unbounded integers specify lengths before usize limits.
No extraction outcome is consulted when selecting input cases or expectations.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent


def length(n, padding):
    return 4 * ((n + 2) // 3) if padding else (8 * n + 5) // 6


def cases(width):
    maximum = (1 << width) - 1
    points = set(range(4097))
    # Binary search the mathematical last representable output independently for
    # each flag, rather than copying the implementation's division/multiply path.
    boundaries = [0, maximum, 3 * (maximum // 4), 3 * (maximum // 4 + 1)]
    for padding in (False, True):
        low, high = 0, maximum + 1
        while low + 1 < high:
            middle = (low + high) // 2
            if length(middle, padding) <= maximum:
                low = middle
            else:
                high = middle
        boundaries.extend([low, high])
    for boundary in boundaries:
        points.update(n for n in range(boundary - 12, boundary + 13)
                      if 0 <= n <= maximum)
    return [(n, padding) for n in sorted(points) for padding in (False, True)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--native')
    parser.add_argument('--pointer-width', type=int, choices=(32, 64), default=64)
    args = parser.parse_args()
    selection = json.loads((ROOT / 'base64-encoded-len-selection.json').read_text())
    source = args.source_root or Path(selection['package']['source_root'])
    for relative, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source / relative).read_bytes()).hexdigest() == digest, relative
    original = (source / selection['function']['file']).read_bytes()
    span = selection['function']['span']
    assert hashlib.sha256(original[span['start']:span['end']]).hexdigest() == selection['function']['sha256']
    rows = cases(args.pointer_width)
    request = ''.join(f'{n} {int(padding)}\n' for n, padding in rows)
    digest = hashlib.sha256(request.encode()).hexdigest()
    frozen = selection['sample'][str(args.pointer_width)]
    assert len(rows) == frozen['cases'] and digest == frozen['request_sha256']
    maximum = (1 << args.pointer_width) - 1
    expected = [f'some {value}' if (value := length(n, padding)) <= maximum else 'none'
                for n, padding in rows]
    if args.native:
        run = subprocess.run([args.native], input=request, text=True,
                             capture_output=True, check=True, timeout=60)
        lines = run.stdout.splitlines()
        assert lines[0] == f'width {args.pointer_width}', lines[:1]
        assert len(lines) == len(rows) + 1, len(lines)
        for row, actual, wanted in zip(rows, lines[1:], expected):
            assert actual == wanted, (row, actual, wanted)
    print(json.dumps({'cases': len(rows), 'pointer_width': args.pointer_width,
                      'native_checked': bool(args.native), 'request_sha256': digest,
                      'some': sum(value.startswith('some') for value in expected),
                      'none': expected.count('none')}, sort_keys=True))


if __name__ == '__main__':
    main()
