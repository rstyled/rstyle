#!/usr/bin/env python3
"""Linux RSS smoke check: more bundled documents with a fixed per-document size.

Measures the Rust receiving process, not the producer or whole frontend. Padding
is source text outside function bodies; this is not a representative graph bench.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def emit(binary, directory, documents, source_mib):
    fixture = Path(__file__).resolve().parents[1] / 'crates/rstyle-core/src/cpg/testdata/example.cpg.json'
    template = json.loads(fixture.read_text())
    source = template['unit']['files'][0]
    source['content'] += '\n/*' + 'x' * (source_mib * 1024 * 1024) + '*/\n'
    source['digest'] = 'sha256:' + hashlib.sha256(source['content'].encode()).hexdigest()
    template = json.dumps(template)
    process = subprocess.Popen(
        [str(binary), 'rstyle', 'cpg', 'ingest', '--out', str(directory)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        for index in range(documents):
            value = json.loads(template.replace('example::', f'sample{index}::')
                               .replace('src/lib.rs', f'sample{index}/lib.rs'))
            value['unit']['id'] = f'unit:sample{index}'
            process.stdin.write(b'\x1e')
            for chunk in json.JSONEncoder(separators=(',', ':')).iterencode(value):
                process.stdin.write(chunk.encode())
            process.stdin.write(b'\n')
        process.stdin.write(b'\x1enull\n')
    finally:
        process.stdin.close()
        process.stdin = None
    _, error = process.communicate()
    if process.returncode:
        raise RuntimeError(error.decode())


def measure(binary, directory, route):
    rss = directory.parent / f'{directory.name}-{route}.rss'
    command = [str(binary), 'rstyle', 'cpg', route, '--capture', str(directory), '--format', 'json']
    started = time.perf_counter()
    result = subprocess.run(['/usr/bin/time', '-f', '%M', '-o', str(rss), *command],
                            capture_output=True, text=True)
    elapsed = time.perf_counter() - started
    if result.returncode:
        raise RuntimeError(f'{route}: {result.stderr}\n{result.stdout}')
    report = json.loads(result.stdout)
    receipt = report if route == 'receive' else report['capture_receipt']
    if receipt['status'] != 'accepted':
        raise RuntimeError(f'Capture rejected: {receipt}')
    (directory.parent / f'{directory.name}-{route}.json').write_text(result.stdout)
    return dict(route=route, peak_rss_kib=int(rss.read_text().strip()),
                elapsed_seconds=elapsed, **receipt['transport'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--documents', type=int, nargs='+', default=[8, 64])
    parser.add_argument('--source-mib', type=int, default=1)
    args = parser.parse_args()
    if args.source_mib < 1 or any(count < 1 for count in args.documents):
        parser.error('source size and document counts must be positive')
    binary = args.binary.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    results = []
    for position, count in enumerate(args.documents):
        directory = args.output / f'capture-{position}-{count}'
        emit(binary, directory, count, args.source_mib)
        for route in ['receive', 'check']:
            result = measure(binary, directory, route)
            if result['verified_documents'] != count:
                raise RuntimeError('Not all documents were verified')
            results.append(result)
            print(json.dumps(result), flush=True)
    (args.output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')


if __name__ == '__main__':
    main()
