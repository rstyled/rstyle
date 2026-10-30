#!/usr/bin/env python3
"""Exact k_cos two-input oracle; native protocol: two hex words in, one out.

Reuses the independent rational IEEE primitive implementation, never host floats.
NaN result-set proof is specific to k_cos and pinned targets without extra payloads:
both original inputs reach late x*y; any original fraction can pass unchanged
through the right correction chain to the final addition, regardless of stable
shared z/hz/w choices. Preferred is available at the final operation. No other
fraction can arise. This shortcut is not a general DAG correlation algorithm.
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
SEED = 0xBB67AE8584CAA73B
DECIMALS = [
    '4.16666666666666019037e-02', '-1.38888888888741095749e-03',
    '2.48015872894767294178e-05', '-2.75573143513906633035e-07',
    '2.08757232129817482790e-09', '-1.13596475577881948265e-11',
]
CONSTANTS = [encode(Fraction(text)) for text in DECIMALS]
FIXED_CONSTANTS = [0x3FA555555555554C, 0xBF56C16C16C15177, 0x3EFA01A019CB1590,
                   0xBE927E4F809C52AD, 0x3E21EE9EBDB4B1C4, 0xBDA8FAE9BE8838D4]


def operation(kind, left, right):
    if left is None or right is None:
        return None
    if kind == 'sub':
        right ^= SIGN
    return BASE['operation']('mul' if kind == 'mul' else 'add', left, right)


def kernel(x, y):
    c1, c2, c3, c4, c5, c6 = CONSTANTS
    add = lambda a, b: operation('add', a, b)
    mul = lambda a, b: operation('mul', a, b)
    sub = lambda a, b: operation('sub', a, b)
    z = mul(x, x)
    w = mul(z, z)
    r = add(mul(z, add(c1, mul(z, add(c2, mul(z, c3))))),
            mul(mul(w, w), add(c4, mul(z, add(c5, mul(z, c6))))))
    hz = mul(0x3FE0000000000000, z)
    w = sub(0x3FF0000000000000, hz)
    return add(w, add(sub(sub(0x3FF0000000000000, w), hz), sub(mul(z, r), mul(x, y))))


def allowed_nan_results(x, y):
    fractions = {1 << 51}
    for value in (x, y):
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
    directed = [(x, y) for x in values for y in values]
    state = SEED
    words = []
    for _ in range(8192):
        state = (state + 0x9E3779B97F4A7C15) & MASK
        value = state
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK
        words.append(value ^ (value >> 31))
    return directed, list(zip(words[::2], words[1::2]))


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
    assert allowed_nan_results(0, 0) == signs({0x7FF8000000000000})
    assert allowed_nan_results(0x7FF0000000000001, 0x7FF8000000000002) == signs({
        0x7FF8000000000000, 0x7FF0000000000001, 0x7FF8000000000001, 0x7FF8000000000002})
    assert allowed_nan_results(0xFFF0000000000003, 0x7FF0000000000001) == signs({
        0x7FF8000000000000, 0x7FF0000000000003, 0x7FF8000000000003,
        0x7FF0000000000001, 0x7FF8000000000001})
    for x, y in [(0, 0), (SIGN, SIGN), (1, 1), (0x0010000000000000, 0)]:
        assert kernel(x, y) == 0x3FF0000000000000
    assert kernel(0, 0x7FF0000000000000) is None
    assert kernel(0x7FF0000000000000, 0) is None
    return len(cases)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native')
    parser.add_argument('--source-root', type=Path, help='unchanged original libm package directory')
    parser.add_argument('--authority', type=Path, help='pinned core/src/primitive_docs.rs; default rustc sysroot')
    args = parser.parse_args()
    primitives = self_test()
    directed, random = vectors()
    selection = json.loads((ROOT / 'float-cos-tail-selection.json').read_text())
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
    assert directed == [(x, y) for x in values for y in values]
    directed_digest = hashlib.sha256(b''.join(x.to_bytes(8, 'big') + y.to_bytes(8, 'big')
                                             for x, y in directed)).hexdigest()
    assert directed_digest == selection['sample']['directed_sha256']
    digest = hashlib.sha256(b''.join(x.to_bytes(8, 'big') + y.to_bytes(8, 'big')
                                     for x, y in random)).hexdigest()
    assert digest == selection['sample']['random_sha256']
    if args.native:
        pairs = directed + random
        request = ''.join(f'{x:016x} {y:016x}\n' for x, y in pairs)
        result = subprocess.run([args.native], input=request, text=True, capture_output=True,
                                check=True, timeout=60)
        lines = result.stdout.splitlines()
        assert len(lines) == len(pairs)
        for (x, y), line in zip(pairs, lines):
            expected, actual = kernel(x, y), int(line, 16)
            assert actual in (allowed_nan_results(x, y) if expected is None else {expected}), (
                hex(x), hex(y), hex(actual), expected)
    print(json.dumps({'primitive_vectors': primitives, 'directed': len(directed),
                      'random': len(random), 'random_sha256': digest,
                      'native_checked': bool(args.native)}, sort_keys=True))


if __name__ == '__main__':
    main()
