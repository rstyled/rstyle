#!/usr/bin/env python3
"""Source-frozen Input borrowed-result oracle, not a constructor admission test.

Valid UTF8 has a string/codepoint boundary contract. Invalid bytes have only the
exact selected implementation's byte-predicate contract; no UTF8 guarantee.
Protocol: hex/start/end/anchor/earliest/offset; output stable/len/hex/boundary/str/distinct.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
TEXTS = ['', 'a', '☃', 'a☃é😀z', '\0', 'é', '😀']
ANCHORS = ['n', 'y', 'p0', 'p1', 'p17', 'pmax']


def sources():
    values = [text.encode() for text in TEXTS]
    values += [bytes([value]) for value in range(256)]
    values += [bytes.fromhex(value) for value in ['8000', '0080', 'c080', 'ffff', '00ff', 'ff00']]
    return list(dict.fromkeys(values))


def requests():
    for source in sources():
        length = len(source)
        spans = list(dict.fromkeys([(0, length), (length, length), (length + 1, length)]))
        for start, end in spans:
            for anchor in ANCHORS:
                for earliest in (0, 1):
                    for offset in [str(i) for i in range(length + 2)] + ['max']:
                        yield source, f'{source.hex() or "-"} {start} {end} {anchor} {earliest} {offset}'


def expected_boundary(source, offset):
    if offset == 'max':
        return False
    index = int(offset)
    # Separate byte law, independent of source match syntax and signed comparison.
    return index == len(source) or index < len(source) and source[index] & 0xC0 != 0x80


def text_boundary(source, offset):
    try:
        text = source.decode('utf-8')
    except UnicodeDecodeError:
        return '-'
    positions = {0}
    position = 0
    for character in text:
        position += len(character.encode('utf-8'))
        positions.add(position)
    return str(int(offset != 'max' and int(offset) in positions))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--native')
    args = parser.parse_args()
    selection = json.loads((ROOT / 'borrowed-result-selection.json').read_text())
    source_root = args.source_root or Path(selection['package']['source_root'])
    for relative, digest in selection['package']['file_sha256'].items():
        assert hashlib.sha256((source_root / relative).read_bytes()).hexdigest() == digest, relative
    rows = list(requests())
    request = '\n'.join(row for _, row in rows) + '\n'
    assert hashlib.sha256(request.encode()).hexdigest() == selection['sample']['request_sha256']
    assert not expected_boundary(bytes.fromhex('80'), '0')
    assert expected_boundary(bytes.fromhex('ff'), '0')
    assert [expected_boundary('☃'.encode(), str(i)) for i in range(5)] == [True, False, False, True, False]
    for source, row in rows:
        offset = row.split()[-1]
        text = text_boundary(source, offset)
        if text != '-':
            assert text == str(int(expected_boundary(source, offset)))
    if args.native:
        run = subprocess.run([args.native], input=request, text=True, capture_output=True,
                             check=True, timeout=60)
        lines = run.stdout.splitlines()
        assert len(lines) == len(rows)
        for (source, row), response in zip(rows, lines):
            offset = row.split()[-1]
            expected = ['1', str(len(source)), source.hex() or '-',
                        str(int(expected_boundary(source, offset))), text_boundary(source, offset), '1']
            assert response.split() == expected, (row, response, expected)
    print(json.dumps({'sources': len(sources()), 'cases': len(rows),
                      'native_checked': bool(args.native),
                      'request_sha256': selection['sample']['request_sha256']}, sort_keys=True))


if __name__ == '__main__':
    main()
