#!/usr/bin/env python3
"""Exact k_sin oracle; native protocol: two hex words and signed i32 in, hex out.

Reuses the independent rational IEEE primitive implementation, never host floats.
NaN proof is branch-specific: iy==0 never evaluates y, so only x payloads
can propagate. For iy!=0 raw x reaches the final subtraction and raw y reaches
a late correction subtraction; each candidate can propagate without conflicting
shared-intermediate choices. Preferred is available at the final operation.
No new payloads occur on pinned targets without extras. Not a general DAG rule.
"""
import argparse
import hashlib
import json
from pathlib import Path
import runpy
import subprocess
from fractions import Fraction

ROOT = Path(__file__).resolve().parent
BASE = runpy.run_path(str(ROOT / 'float-kernels-oracle.py'))
encode, decode, power = (BASE[key] for key in ('encode', 'decode', 'power'))
SIGN = 1 << 63
MASK = (1 << 64) - 1
SEED = 0x3C6EF372FE94F82B
IYS = [0, 1, -1, -(1 << 31), (1 << 31) - 1]
EXTRA = [(x, y, iy) for x in (0x3E40000000000000, 0xBE40000000000000)
         for y in (0x3AE0000000000000, 0xBAE0000000000000) for iy in (0, 1)]
DECIMALS = [
    '-1.66666666666666324348e-01', '8.33333333332248946124e-03',
    '-1.98412698298579493134e-04', '2.75573137070700676789e-06',
    '-2.50507602534068634195e-08', '1.58969099521155010221e-10',
]
CONSTANTS = [encode(Fraction(text)) for text in DECIMALS]
FIXED_CONSTANTS = [0xBFC5555555555549, 0x3F8111111110F8A6, 0xBF2A01A019C161D5,
                   0x3EC71DE357B1FE7D, 0xBE5AE5E68A2B9CEB, 0x3DE5D93A5ACFD57C]


def operation(kind, left, right):
    if left is None or right is None:
        return None
    if kind == 'sub':
        right ^= SIGN
    return BASE['operation']('mul' if kind == 'mul' else 'add', left, right)


def kernel(x, y, iy):
    s1, s2, s3, s4, s5, s6 = CONSTANTS
    add = lambda a, b: operation('add', a, b)
    mul = lambda a, b: operation('mul', a, b)
    sub = lambda a, b: operation('sub', a, b)
    z = mul(x, x)
    w = mul(z, z)
    r = add(add(s2, mul(z, add(s3, mul(z, s4)))),
            mul(mul(z, w), add(s5, mul(z, s6))))
    v = mul(z, x)
    if iy == 0:
        return add(x, mul(v, add(s1, mul(z, r))))
    return sub(x, sub(sub(mul(z, sub(mul(0x3FE0000000000000, y), mul(v, r))), y), mul(v, s1)))


def allowed_nan_results(x, y, iy):
    fractions = {1 << 51}
    for value in ((x,) if iy == 0 else (x, y)):
        if decode(value)[0] == 'nan':
            fraction = value & ((1 << 52) - 1)
            fractions.update((fraction, fraction | (1 << 51)))
    return {0x7FF0000000000000 | fraction | sign
            for fraction in fractions for sign in (0, SIGN)}


def vectors():
    values = [0, SIGN, 1, SIGN | 1, 0x000FFFFFFFFFFFFF, 0x0010000000000000,
              0x3FE0000000000000, 0x3FEFFFFFFFFFFFFF, 0x3FF0000000000000,
              0x7FEFFFFFFFFFFFFF, 0x7FF0000000000000, 0xFFF0000000000000,
              0x7FF0000000000001, 0x7FF8000000000002, 0xFFF0000000000003,
              0xFFFFFFFFFFFFFFFF]
    directed = [(x, y, iy) for x in values for y in values for iy in IYS]
    state = SEED
    words = []
    for _ in range(12288):
        state = (state + 0x9E3779B97F4A7C15) & MASK
        value = state
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK
        words.append(value ^ (value >> 31))
    random = [(words[i], words[i + 1], ((words[i + 2] & 0xFFFFFFFF) ^ (1 << 31)) - (1 << 31))
              for i in range(0, len(words), 3)]
    return directed + EXTRA, random


def self_test():
    BASE['self_test']()
    assert CONSTANTS == FIXED_CONSTANTS
    cases = [
        ('sub', 0, 0, 0), ('sub', SIGN, 0, SIGN), ('sub', 0, SIGN, 0),
        ('sub', SIGN, SIGN, 0), ('sub', 0x3FF0000000000000, 0x3FF0000000000000, 0),
        ('sub', 1, 0x8000000000000001, 2),
        ('sub', 0x7FEFFFFFFFFFFFFF, 0xFFEFFFFFFFFFFFFF, 0x7FF0000000000000),
        ('sub', 0x7FF0000000000000, 0x7FF0000000000000, None),
        ('mul', 0, 0x7FF0000000000000, None),
        ('add', 0x3FF0000000000000, 0x3CA0000000000000, 0x3FF0000000000000),
    ]
    for kind, a, b, expected in cases:
        assert operation(kind, a, b) == expected, (kind, hex(a), hex(b))
    signs = lambda values: {v | sign for v in values for sign in (0, SIGN)}
    assert allowed_nan_results(0, 0, 1) == signs({0x7FF8000000000000})
    assert allowed_nan_results(0x7FF0000000000001, 0x7FF8000000000002, 1) == signs({
        0x7FF8000000000000, 0x7FF0000000000001, 0x7FF8000000000001, 0x7FF8000000000002})
    assert allowed_nan_results(0xFFF0000000000003, 0x7FF0000000000001, -1) == signs({
        0x7FF8000000000000, 0x7FF0000000000003, 0x7FF8000000000003,
        0x7FF0000000000001, 0x7FF8000000000001})
    for x in (0, SIGN):
        for y in (0, SIGN):
            assert kernel(x, y, 0) == 0
            assert kernel(x, y, 1) == x
    for x in (0, SIGN, 1, 0x3FE0000000000000, 0x7FF0000000000000):
        assert kernel(x, 0x7FF0000000000123, 0) == kernel(x, 0, 0)
    assert allowed_nan_results(0x7FF0000000000001, 0x7FF8000000000002, 0) == signs({
        0x7FF8000000000000, 0x7FF0000000000001, 0x7FF8000000000001})
    assert kernel(0x7FF0000000000000, 0, 0) == 0x7FF0000000000000
    assert kernel(0xFFF0000000000000, 0, 0) == 0xFFF0000000000000
    assert kernel(0, 0x7FF0000000000000, 1) is None
    assert kernel(0x7FF0000000000000, 0, 1) is None
    for x, y in [(0, 0), (SIGN, SIGN), (1, 1), (0x3FE0000000000000, 0x3CA0000000000000)]:
        assert len({kernel(x, y, iy) for iy in IYS if iy}) == 1
    return len(cases)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native')
    parser.add_argument('--source-root', type=Path, help='unchanged original libm package directory')
    parser.add_argument('--authority', type=Path, help='pinned core/src/primitive_docs.rs; default rustc sysroot')
    args = parser.parse_args()
    primitives = self_test()
    directed, random = vectors()
    selection = json.loads((ROOT / 'float-sin-tail-selection.json').read_text())
    source_root = args.source_root or Path(selection['package']['source_root'])
    for relative, expected_digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source_root / relative).read_bytes()).hexdigest() == expected_digest, relative
    original = source_root / selection['function']['source']
    assert hashlib.sha256(original.read_bytes()).hexdigest() == selection['function']['file_sha256']
    if args.authority:
        authority = args.authority
    else:
        sysroot = subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip()
        authority = Path(sysroot) / 'lib/rustlib/src/rust/library/core/src/primitive_docs.rs'
    assert hashlib.sha256(authority.read_bytes()).hexdigest() == selection['nan_authority']['sha256']
    values = [int(value, 16) for value in selection['sample']['directed_values']]
    extras = [(int(x, 16), int(y, 16), iy) for x, y, iy in selection['sample']['extra_triples']]
    assert directed == [(x, y, iy) for x in values for y in values for iy in selection['sample']['directed_iy']] + extras
    directed_digest = hashlib.sha256(b''.join(x.to_bytes(8, 'big') + y.to_bytes(8, 'big') + iy.to_bytes(4, 'big', signed=True)
                                             for x, y, iy in directed)).hexdigest()
    assert directed_digest == selection['sample']['directed_sha256']
    digest = hashlib.sha256(b''.join(x.to_bytes(8, 'big') + y.to_bytes(8, 'big') + iy.to_bytes(4, 'big', signed=True)
                                     for x, y, iy in random)).hexdigest()
    assert digest == selection['sample']['random_sha256']
    if args.native:
        pairs = directed + random
        request = ''.join(f'{x:016x} {y:016x} {iy}\n' for x, y, iy in pairs)
        result = subprocess.run([args.native], input=request, text=True, capture_output=True,
                                check=True, timeout=60)
        lines = result.stdout.splitlines()
        assert len(lines) == len(pairs)
        for (x, y, iy), line in zip(pairs, lines):
            expected, actual = kernel(x, y, iy), int(line, 16)
            assert actual in (allowed_nan_results(x, y, iy) if expected is None else {expected}), (
                hex(x), hex(y), iy, hex(actual), expected)
    print(json.dumps({'primitive_vectors': primitives, 'directed': len(directed),
                      'random': len(random), 'random_sha256': digest,
                      'native_checked': bool(args.native)}, sort_keys=True))


if __name__ == '__main__':
    main()
