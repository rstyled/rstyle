#!/usr/bin/env python3
"""Independent integer expectations; full-package and private-source runs differ."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def cases(width, bounded):
    maximum = (1 << width) - 1
    if bounded:
        points = range(65537)
    else:
        points = set(range(4097))
        for boundary in [maximum // 4, 3 * (maximum // 4), maximum]:
            points.update(n for n in range(boundary - 12, boundary + 13)
                          if 0 <= n <= maximum)
    return [(n, padded) for n in sorted(points) for padded in (False, True)]


def lengths(n, padded, width):
    maximum = (1 << width) - 1
    # Integer ceilings specify the result without copying the source's match,
    # remainder cast, bitmask or arithmetic evaluation order.
    size = 4 * ((n + 2) // 3) if padded else (4 * n + 2) // 3
    ct = None if 4 * n > maximum else size
    peer = None if size > maximum else size
    return ct, peer


def token(value):
    return 'none' if value is None else f'some:{value}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--pointer-width', type=int, choices=(32, 64), default=64)
    parser.add_argument('--native', help='unchanged full packages, bounded actual slices')
    parser.add_argument('--helper-native', help='separate exact private-helper source witness')
    parser.add_argument('--write-helper', type=Path, help='export exact frozen original helper bytes')
    args = parser.parse_args()
    assert not (args.native and args.helper_native), 'choose one evidence context'
    selection = json.loads(Path(__file__).with_name('base64ct-length-selection.json').read_text())
    root = args.source_root or Path(selection['package']['source_root'])
    for name, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((root / name).read_bytes()).hexdigest() == digest, name
    function = selection['function']
    source = (root / function['file']).read_bytes()
    span = function['span']
    helper = source[span['start']:span['end']]
    assert hashlib.sha256(helper).hexdigest() == function['sha256']
    peer = selection['comparison_peer']
    peer_root = Path(peer['package']['source_root'])
    for name, digest in peer['package']['file_sha256'].items():
        assert hashlib.sha256((peer_root / name).read_bytes()).hexdigest() == digest, name
    peer_function = peer['function']
    peer_source = (peer_root / peer_function['file']).read_bytes()
    peer_span = peer_function['span']
    assert hashlib.sha256(peer_source[peer_span['start']:peer_span['end']]).hexdigest() == peer_function['sha256']
    if args.write_helper:
        args.write_helper.write_bytes(helper)
    rows = cases(args.pointer_width, bool(args.native))
    request = ''.join(f'{n} {int(padded)}\n' for n, padded in rows)
    expected = [' '.join(token(value) for value in lengths(n, padded, args.pointer_width))
                for n, padded in rows]
    executable = args.native or args.helper_native
    if executable:
        run = subprocess.run([executable], input=request, text=True,
                             capture_output=True, check=True, timeout=60)
        lines = run.stdout.splitlines()
        assert lines[0] == f'width:{args.pointer_width}'
        assert lines[1:] == expected, 'original execution differs from integer oracle'
    print(json.dumps({'source_identity': 'verified', 'cases': len(rows),
                      'request_sha256': hashlib.sha256(request.encode()).hexdigest(),
                      'execution': 'full-package bounded' if args.native else
                      'private-source witness' if args.helper_native else 'not performed',
                      'incompatible_peer_cases': sum(left != right for n, padded in rows
                          for left, right in [lengths(n, padded, args.pointer_width)])}))


if __name__ == '__main__':
    main()
