"""Retain exact captured bytes, compression only; establish no new conformance."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path


def retain(source, destination):
    data = source.read_bytes()
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(gzip.compress(data, mtime=0))
    return dict(path=str(destination), original_sha256=hashlib.sha256(data).hexdigest(), compressed_sha256=hashlib.sha256(destination.read_bytes()).hexdigest(), original_bytes=len(data))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('stages', nargs='+')
    args = parser.parse_args()
    inventory = Path(__file__).resolve().parent
    selection = json.loads((inventory/'selection.json').read_text())
    packages = sorted({(f['package'], f['version']) for f in selection})
    evidence = []
    for stage in args.stages:
        for name, version in packages:
            package = args.root/f'{name}-{version}'
            records = []
            for suffix in ['stdout.json', 'stderr.txt']:
                source = package/f'{stage}.{suffix}'
                if source.exists():
                    record = retain(source, inventory/'retained'/stage/f'{name}-{version}.{suffix}.gz')
                    record['path'] = str(Path(record['path']).relative_to(inventory))
                    records.append(record)
            report_path = package/f'{stage}.stdout.json'
            report = json.loads(report_path.read_text()) if report_path.exists() and report_path.stat().st_size else {}
            for function in report.get('functions', []):
                if function['outcome'] != 'admitted':
                    continue
                facts = function['facts']
                selected = [f for f in selection if f['package'] == name and facts['build']['frontend_options'].get('source_file', '').endswith('/'+f['path']) and f['start'] <= facts['fn_span']['start'] < f['end']]
                if len(selected) != 1:
                    continue
                prefix = Path(function['artifact_prefix'])
                for suffix in ['assessment.json', 'manifest.json', 'pdg.json', 'cfg.json']:
                    source = Path(str(prefix)+'.'+suffix)
                    record = retain(source, inventory/'retained'/stage/f'{name}-{version}'/f'{selected[0]["start"]}.{suffix}.gz')
                    record.update(path=str(Path(record['path']).relative_to(inventory)), function_key=function['key'], selected_source_sha256=selected[0]['sha256'])
                    records.append(record)
            evidence.append(dict(stage=stage, package=name, version=version, artifacts=records))
    (inventory/'retained-index.json').write_text(json.dumps(evidence, indent=2)+'\n')


if __name__ == '__main__':
    main()
