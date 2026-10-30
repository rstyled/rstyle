#!/usr/bin/env python3
"""Independent exact-dyadic oracle. Native protocol: JSON lines in/out.

Invoke: python3 float-kernels-oracle.py --native /path/to/harness
Harness reads {"input_bits":"<16 hex>"}, returns {"sin_bits":"<8 hex>",
"cos_bits":"<8 hex>"}, evaluating unchanged original files included as modules.
No std sin/cos, host float arithmetic or producer code is used by this oracle.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from fractions import Fraction

ROOT = Path(__file__).resolve().parent
MASK = (1 << 64) - 1
SEED = 0x6A09E667F3BCC909


def rounded_integer(value):
    q, r = divmod(value.numerator, value.denominator)
    return q + (2 * r > value.denominator or
                (2 * r == value.denominator and q % 2 != 0))


def power(exponent):
    return Fraction(1 << exponent) if exponent >= 0 else Fraction(1, 1 << -exponent)


def encode(value, width=64, negative_zero=False):
    """Round exact finite rational to IEEE nearest/even (including overflow)."""
    eb, fb = (11, 52) if width == 64 else (8, 23)
    bias = (1 << (eb - 1)) - 1
    sign = value < 0 or value == 0 and negative_zero
    value = abs(value)
    sb = int(sign) << (width - 1)
    if value == 0:
        return sb
    exponent = value.numerator.bit_length() - value.denominator.bit_length()
    if value < power(exponent):
        exponent -= 1
    if exponent < 1 - bias:
        significand = rounded_integer(value / power(1 - bias - fb))
        return sb | significand
    significand = rounded_integer(value / power(exponent - fb))
    if significand == 1 << (fb + 1):
        significand >>= 1
        exponent += 1
    if exponent > bias:
        return sb | (((1 << eb) - 1) << fb)
    return sb | ((exponent + bias) << fb) | (significand - (1 << fb))


def decode(bits, width=64):
    eb, fb = (11, 52) if width == 64 else (8, 23)
    bias = (1 << (eb - 1)) - 1
    exponent = (bits >> fb) & ((1 << eb) - 1)
    fraction = bits & ((1 << fb) - 1)
    sign = bool(bits >> (width - 1))
    if exponent == (1 << eb) - 1:
        return ('nan' if fraction else 'inf', sign, None)
    integer = fraction if exponent == 0 else fraction + (1 << fb)
    value = integer * power((1 if exponent == 0 else exponent) - bias - fb)
    return ('finite', sign, -value if sign else value)


def operation(op, left, right):
    a, sa, va = decode(left)
    b, sb, vb = decode(right)
    # NaN is an abstract classification, never an invented canonical payload.
    if a == 'nan' or b == 'nan':
        return None
    if op == 'mul':
        if a == 'inf' or b == 'inf':
            if va == 0 or vb == 0:
                return None
            return (int(sa != sb) << 63) | 0x7FF0000000000000
        return encode(va * vb, negative_zero=sa != sb)
    if a == 'inf' or b == 'inf':
        if a == b == 'inf' and sa != sb:
            return None
        return left if a == 'inf' else right
    return encode(va + vb, negative_zero=sa and sb)


def op(kind, a, b):
    return None if a is None or b is None else operation(kind, a, b)


def narrow(bits):
    if bits is None:
        return None
    kind, sign, value = decode(bits)
    if kind == 'nan':
        return None
    if kind == 'inf':
        return (int(sign) << 31) | 0x7F800000
    return encode(value, 32, sign)


DECIMALS = {
    'S1': '-0.166666666416265235595', 'S2': '0.0083333293858894631756',
    'S3': '-0.000198393348360966317347', 'S4': '0.0000027183114939898219064',
    'C0': '-0.499999997251031003120', 'C1': '0.0416666233237390631894',
    'C2': '-0.00138867637746099294692', 'C3': '0.0000243904487962774090654',
}
CONSTANTS = {name: encode(Fraction(text)) for name, text in DECIMALS.items()}


def kernels(x):
    c = CONSTANTS
    z = op('mul', x, x)
    w = op('mul', z, z)
    r = op('add', c['S3'], op('mul', z, c['S4']))
    s = op('mul', z, x)
    sine = op('add', op('add', x, op('mul', s, op('add', c['S1'], op('mul', z, c['S2'])))),
              op('mul', op('mul', s, w), r))
    r = op('add', c['C2'], op('mul', z, c['C3']))
    cosine = op('add', op('add', op('add', encode(Fraction(1)), op('mul', z, c['C0'])),
                                  op('mul', w, c['C1'])), op('mul', op('mul', w, z), r))
    return narrow(sine), narrow(cosine)


def allowed_nan_results(input_bits):
    """Exact final f32 NaN set on pinned aarch64/x86_64 (no extra payloads).

    All kernel paths perform arithmetic before the narrowing cast. Intermediate
    arithmetic can only select preferred or propagate/quiet the original NaN.
    Either propagated state can reach the final operation along a dependency
    path, so stable shared intermediates do not remove any member of this set.
    """
    payloads = {0x7FC00000}
    kind, _, _ = decode(input_bits)
    if kind == 'nan':
        fraction = input_bits & ((1 << 52) - 1)
        quiet = bool(fraction & (1 << 51))
        narrowed_payload = (fraction & ((1 << 51) - 1)) >> 29
        payloads.add(0x7FC00000 | narrowed_payload)
        unchanged = 0x7F800000 | (int(quiet) << 22) | narrowed_payload
        if unchanged & ((1 << 23) - 1):
            payloads.add(unchanged)
    return {bits | sign for bits in payloads for sign in [0, 1 << 31]}


def vectors():
    directed = [0, 1 << 63, 1, (1 << 52) - 1, 1 << 52,
                0x7FEFFFFFFFFFFFFF, 0x7FF0000000000000,
                0x7FF8000000000001, 0x7FF0000000000001,
                0x7FF000001FFFFFFF, 0x7FF0000020000000,
                0x7FF8000020000000, 0x7FFFFFFFFFFFFFFF,
                0xFFF0000020000000, 0xFFF8000020000000]
    for exponent in [-1074, -1022, -150, -149, -126, -25, -24, -1, 0, 1, 24, 127, 128, 511, 512, 1023]:
        center = encode(power(exponent))
        for delta in [-1, 0, 1]:
            directed.extend([center + delta, (center + delta) | (1 << 63)])
    state = SEED
    random = []
    for _ in range(4096):
        # SplitMix64 with fixed arithmetic, independent of Python random versions.
        state = (state + 0x9E3779B97F4A7C15) & MASK
        value = state
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK
        random.append(value ^ (value >> 31))
    return list(dict.fromkeys(directed)), random


def self_test():
    cases = [
        (Fraction(0), 64, False, 0), (Fraction(0), 64, True, 1 << 63),
        (Fraction(1), 64, False, 0x3FF0000000000000),
        (Fraction(-1), 32, False, 0xBF800000),
        (power(-1074), 64, False, 1), (power(-1075), 64, False, 0),
        (3 * power(-1075), 64, False, 2),
        (power(-149), 32, False, 1), (power(-150), 32, False, 0),
        (3 * power(-150), 32, False, 2),
        (1 + power(-24), 32, False, 0x3F800000),
        (1 + 3 * power(-24), 32, False, 0x3F800002),
        (power(128) - power(103), 32, False, 0x7F800000),
        (power(128) - power(103) - 1, 32, False, 0x7F7FFFFF),
    ]
    for value, width, negative, expected in cases:
        assert encode(value, width, negative) == expected, (value, width)
    assert kernels(0) == kernels(1 << 63) == (0, 0x3F800000)
    assert kernels(0x7FF0000000000000) == (0x7F800000, None)
    assert kernels(0xFFF0000000000000) == (0xFF800000, None)
    # Fixed independently specified sets exercise the 51 -> 22 payload truncation.
    signs = lambda values: {value | sign for value in values for sign in [0, 1 << 31]}
    assert allowed_nan_results(0) == signs({0x7FC00000})
    assert allowed_nan_results(0x7FF0000000000001) == signs({0x7FC00000})
    assert allowed_nan_results(0x7FF000001FFFFFFF) == signs({0x7FC00000})
    assert allowed_nan_results(0x7FF0000020000000) == signs({0x7FC00000, 0x7FC00001, 0x7F800001})
    assert allowed_nan_results(0x7FF8000020000000) == signs({0x7FC00000, 0x7FC00001})
    assert allowed_nan_results(0x7FFFFFFFFFFFFFFF) == signs({0x7FC00000, 0x7FFFFFFF})
    assert allowed_nan_results(0xFFF0000020000000) == allowed_nan_results(0x7FF0000020000000)
    return len(cases)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native', help='native unchanged-original-file JSONL harness executable')
    args = parser.parse_args()
    primitives = self_test()
    directed, random = vectors()
    expected = [(bits, kernels(bits)) for bits in directed + random]
    if args.native:
        request = ''.join(json.dumps({'input_bits': f'{bits:016x}'}) + '\n' for bits, _ in expected)
        run = subprocess.run([args.native], input=request, text=True, capture_output=True,
                             check=True, timeout=60)
        responses = run.stdout.splitlines()
        assert len(responses) == len(expected), 'native response count differs'
        for (bits, pair), line in zip(expected, responses):
            response = json.loads(line)
            for field, reference in zip(['sin_bits', 'cos_bits'], pair):
                actual = int(response[field], 16)
                if reference is None:
                    assert actual in allowed_nan_results(bits), (hex(bits), field, response,
                                                                sorted(allowed_nan_results(bits)))
                else:
                    assert actual == reference, (hex(bits), field, hex(reference), hex(actual))
    checksum = hashlib.sha256(b''.join(bits.to_bytes(8, 'big') for bits in random)).hexdigest()
    print(json.dumps({'primitive_vectors': primitives, 'directed': len(directed),
                      'frozen_random': len(random), 'random_sha256': checksum,
                      'native_checked': bool(args.native),
                      'nan_payload_authority': 'pinned Rust primitive_docs; exact set for aarch64/x86_64',
                      'constant_bits': {key: f'{value:016x}' for key, value in CONSTANTS.items()}}, sort_keys=True))


if __name__ == '__main__':
    main()
