#!/usr/bin/env python3
"""Share a graph-rich capture while retaining independent numeric/pattern CI gates."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys


def invoke(command, output):
    with output.open('wb') as stream:
        try:
            return subprocess.run(command, stdout=stream, check=False).returncode
        except OSError as error:
            print(f'ci-style: {error}', file=sys.stderr)
            return 2


def summarize(gate, code, output, error=None):
    result = ('passed', 'findings', 'tool error')[code]
    print(f'ci-style: {gate}: {result}; reports: {output}')
    if error:
        print(f'ci-style: {error}', file=sys.stderr)


def require_readable(path):
    if not path.is_file():
        raise ValueError(f'missing or non-regular report: {path}')
    with path.open('rb') as stream:
        stream.read(0)


def collect(binary, output):
    patterns = numeric = 2
    error = None
    try:
        output.mkdir(parents=True, exist_ok=False)
    except OSError as failure:
        print(f'ci-style: output directory must be new and writable: {failure}', file=sys.stderr)
        return 2
    try:
        pattern_report = output / 'rstyle-patterns.json'
        patterns = invoke([str(binary), 'rstyle', '--format', 'json', 'patterns',
                           '--fail-on-candidates', '--show-ignored'], pattern_report)
        if patterns in (0, 1):
            # Candidate findings do not invalidate the captured evidence.
            report = json.loads(pattern_report.read_text())
            directory = Path(report['run_directory'])
            shutil.copyfile(directory / 'report.json', output / 'patterns-capture-report.json')
            numeric = invoke([str(binary), 'rstyle', '--format', 'json', '--replay-run',
                              str(directory)], output / 'rstyle-report.json')
        else:
            error = 'Pattern capture failed; numeric gate cannot certify its inputs'
    except (OSError, ValueError, KeyError, TypeError) as failure:
        error = str(failure)
    numeric = numeric if numeric in (0, 1) else 2
    patterns = patterns if patterns in (0, 1) else 2
    try:
        if not (output / 'rstyle-report.json').exists():
            (output / 'rstyle-report.json').write_text(json.dumps({'error': error}) + '\n')
        status = {'numeric': numeric, 'patterns': patterns, 'error': error}
        (output / 'status.json').write_text(json.dumps(status, indent=2) + '\n')
    except OSError as failure:
        print(f'ci-style: cannot retain gate status: {failure}', file=sys.stderr)
        return 2
    summarize("numeric", numeric, output, error)
    return numeric


def saved_patterns(output):
    try:
        status = json.loads((output / 'status.json').read_text())
        if not isinstance(status, dict):
            raise ValueError('invalid gate status object')
        for key in ('numeric', 'patterns'):
            if type(status[key]) is not int or status[key] not in (0, 1, 2):
                raise ValueError(f'invalid {key} exit status')
        require_readable(output / 'rstyle-report.json')
        require_readable(output / 'rstyle-patterns.json')
        if status['patterns'] in (0, 1):
            require_readable(output / 'patterns-capture-report.json')
        error = status.get('error')
        if error is not None and not isinstance(error, str):
            raise ValueError('invalid saved error cause')
        summarize('patterns', status['patterns'], output, error)
        return status['patterns']
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'ci-style: cannot read saved pattern gate: {error}', file=sys.stderr)
        return 2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--patterns-status', action='store_true')
    args = parser.parse_args()
    if args.patterns_status:
        return saved_patterns(args.output)
    if args.binary is None:
        parser.error('--binary is required for capture')
    return collect(args.binary.resolve(), args.output)


if __name__ == '__main__':
    sys.exit(main())
