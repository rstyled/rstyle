#!/usr/bin/env python3
"""Independent bigint oracle for exact ryu private-source multiply/shift spans.

python3 oracle.py --prepare /tmp/ryu-constant-private.rs
RSTYLE_PRIVATE_SOURCE=/tmp/ryu-constant-private.rs rustc --edition=2021 \
  -C debug-assertions=on -C overflow-checks=on -C panic=unwind \
  native.rs -o /tmp/ryu-constant-witness
python3 oracle.py --native /tmp/ryu-constant-witness

This witness is not execution through the unchanged complete ryu library.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import random
import subprocess

ROOT = Path(__file__).resolve().parent
U32_MAX = (1 << 32) - 1
U64_MAX = (1 << 64) - 1
SHIFTS32 = (-2147483648, -1, 0, 31, 32, 33, 63, 64, 95, 96, 127, 2147483647)
SHIFTS64 = (0, 31, 32, 33, 63, 64, 65, 95, 96, 127, 128, 191, 192, U32_MAX)


def boundaries(bits):
    maximum = (1 << bits) - 1
    values = {0, 1, maximum - 1, maximum}
    for exponent in range(bits):
        power = 1 << exponent
        values.update(v for v in (power - 1, power, power + 1) if v <= maximum)
    return sorted(values)


def cases():
    # Cartesian endpoints and representative powers; all powers/adjacencies are
    # independently exercised against extreme and mixed counterpart operands.
    ms = boundaries(32)
    factors = boundaries(64)
    for m in ms:
        for factor in (0, 1, (1 << 32) - 1, 1 << 32, 1 << 63, U64_MAX):
            for shift in SHIFTS32:
                yield (32, m, factor, shift)
    for factor in factors:
        for m in (0, 1, (1 << 31), U32_MAX):
            for shift in SHIFTS32:
                yield (32, m, factor, shift)
    # Boundary of final assertion: quotient crosses 2^32 here. No split product.
    for m in (1, 2, 3, (1 << 16) - 1, 1 << 16, U32_MAX):
        for shift in (33, 34, 63, 64, 65, 95):
            threshold = ((1 << (32 + shift)) + m - 1) // m
            for factor in (threshold - 1, threshold, threshold + 1):
                if 0 <= factor <= U64_MAX:
                    yield (32, m, factor, shift)
    for value in boundaries(64):
        for shift in SHIFTS64:
            yield (64, value, U64_MAX, 1, shift)
            yield (64, U64_MAX, value, U64_MAX, shift)
    rng = random.Random(0x525955434F4E5354)
    for _ in range(8192):
        yield (32, rng.getrandbits(32), rng.getrandbits(64), rng.choice(SHIFTS32))
        yield (32, rng.getrandbits(32), rng.getrandbits(64), rng.randrange(33, 96))
        yield (64, rng.getrandbits(64), rng.getrandbits(64),
               rng.getrandbits(64), rng.choice(SHIFTS64))


def expected(row):
    if row[0] == 32:
        _, m, factor, shift = row
        if shift <= 32 or shift >= 96:
            return 'panic'
        quotient = (m * factor) >> shift
        return f'normal {quotient}' if quotient <= U32_MAX else 'panic'
    _, m, lo, hi, shift = row
    if shift < 64 or shift >= 192:
        return 'panic'
    product = m * (lo + (1 << 64) * hi)
    return f'normal {(product >> shift) & U64_MAX}'


def sources(selection, source):
    for name, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source / name).read_bytes()).hexdigest() == digest, name
    for function in selection['negative_controls']:
        raw = (source / function['file']).read_bytes()
        span = function['span']
        snippet = raw[span['start']:span['end']]
        assert hashlib.sha256(snippet).hexdigest() == function['sha256'], function['name']
        assert snippet.decode() == function['source'], function['name']
    snippets = []
    for function in selection['functions']:
        raw = (source / function['file']).read_bytes()
        span = function['span']
        snippet = raw[span['start']:span['end']]
        assert hashlib.sha256(snippet).hexdigest() == function['sha256'], function['name']
        assert snippet.decode() == function['source'], function['name']
        snippets.append(snippet)
    return b'\n\n'.join(snippets) + b'\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--prepare', type=Path)
    parser.add_argument('--native', type=Path)
    args = parser.parse_args()
    selection = json.loads((ROOT.parent / 'primitive-constant-selection.json').read_text())
    source = args.source_root or Path(selection['package']['source_root'])
    private_source = sources(selection, source)
    if args.prepare:
        args.prepare.write_bytes(private_source)
    rows = list(dict.fromkeys(cases()))
    if args.native:
        request = ''.join(' '.join(map(str, row)) + '\n' for row in rows)
        run = subprocess.run([str(args.native.resolve())], input=request, text=True,
                             capture_output=True, check=True, timeout=90)
        lines = run.stdout.splitlines()
        assert lines[0] == 'PRIVATE-SOURCE debug_assertions=true overflow_checks=true width=64'
        assert len(lines) == len(rows) + 1, (len(lines), len(rows))
        for row, actual in zip(rows, lines[1:]):
            assert actual == expected(row), (row, actual, expected(row))
    counts = Counter((row[0], expected(row).split()[0]) for row in rows)
    print(json.dumps({
        'witness': 'PRIVATE-SOURCE; not unchanged full-library execution',
        'native_checked': bool(args.native), 'cases': len(rows),
        'target_normal': counts[32, 'normal'], 'target_panic': counts[32, 'panic'],
        'anchor_normal': counts[64, 'normal'], 'anchor_panic': counts[64, 'panic'],
        'seeded_random_iterations': 8192,
        'shift_type': 'i32 target; u32 anchor',
        'oracle': 'Python unbounded multiplication and shift; no split arithmetic',
    }, indent=2))


if __name__ == '__main__':
    main()
