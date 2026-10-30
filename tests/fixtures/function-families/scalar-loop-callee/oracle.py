#!/usr/bin/env python3
"""Independent divisibility oracle for the selected ryu private-source witness.

Prepare exact source-hashed spans with --prepare /tmp/ryu-private.rs, compile
native.rs with RSTYLE_PRIVATE_SOURCE pointing there (see its build instructions),
then pass --native /tmp/ryu-private-oracle. This does not build or capture code.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
MAXIMUM = (1 << 32) - 1
# Python integers are unbounded. Include the first unrepresentable power too.
POWERS = tuple(5 ** exponent for exponent in range(15))


def factor(value):
    assert 0 < value <= MAXIMUM
    # Mathematical divisibility against independently precomputed powers, not
    # the implementation's stateful repeated division/counting recurrence.
    return max(exponent for exponent, power in enumerate(POWERS)
               if value % power == 0)


def cases():
    values = set(range(4096))
    for power in POWERS:
        values.update(value for value in range(power - 2, power + 3)
                      if 0 <= value <= MAXIMUM)
    values.update((MAXIMUM, MAXIMUM - 1, MAXIMUM - 2))
    state = 0x52595531
    for _ in range(4096):
        state = (1664525 * state + 1013904223) & MAXIMUM
        values.add(state)
    for value in sorted(values):
        k = factor(value) if value else 0
        for p in sorted({0, k, k + 1, 13, 14, MAXIMUM}):
            yield value, p


def expected(value, p):
    if value == 0:
        return 'panic panic'
    k = factor(value)
    # Caller truth is specified directly by divisibility, independently of the
    # helper's expected count. p>=14 cannot divide any positive u32 value.
    result = p < len(POWERS) and value % POWERS[p] == 0
    assert result == (k >= p)
    return f'normal {k} {str(result).lower()}'


def sources(selection, source):
    for name, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source / name).read_bytes()).hexdigest() == digest, name
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
    parser.add_argument('--prepare', type=Path, help='write verified original private spans')
    parser.add_argument('--native', type=Path)
    args = parser.parse_args()
    selection = json.loads((ROOT.parent / 'scalar-loop-callee-selection.json').read_text())
    source = args.source_root or Path(selection['package']['source_root'])
    private_source = sources(selection, source)
    if args.prepare:
        args.prepare.write_bytes(private_source)
    rows = list(cases())
    request = ''.join(f'{value} {p}\n' for value, p in rows)
    normal = sum(value != 0 for value, _ in rows)
    panic = sum(value == 0 for value, _ in rows)
    if args.native:
        run = subprocess.run([str(args.native.resolve())], input=request, text=True,
                             capture_output=True, check=True, timeout=60)
        lines = run.stdout.splitlines()
        assert lines[0] == 'PRIVATE-SOURCE debug_assertions=true width=64', lines[:1]
        assert len(lines) == len(rows) + 1, len(lines)
        for (value, p), actual in zip(rows, lines[1:]):
            wanted = expected(value, p)
            assert actual == wanted, (value, p, actual, wanted)
    print(json.dumps({
        'witness': 'PRIVATE-SOURCE; not unchanged full-library execution',
        'native_checked': bool(args.native), 'cases': len(rows),
        'distinct_values': len({value for value, _ in rows}),
        'exhaustive_values': '0..=4095', 'deterministic_samples': 4096,
        'helper_normal': normal, 'helper_panic': panic,
        'caller_normal': normal, 'caller_panic': panic,
        'caller_true': sum(value != 0 and expected(value, p).endswith(' true')
                           for value, p in rows),
        'caller_false': sum(value != 0 and expected(value, p).endswith(' false')
                            for value, p in rows),
        'zero_p0_expected': 'panic panic',
        'request_sha256': hashlib.sha256(request.encode()).hexdigest(),
        'private_source_sha256': hashlib.sha256(private_source).hexdigest(),
    }, sort_keys=True))


if __name__ == '__main__':
    main()
