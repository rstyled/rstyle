#!/usr/bin/env python3
"""Audit preserved CCGraph v2 evidence and package bounded explanatory traces."""
import pathlib
import ccgraph_followup as f
import ccgraph_study as b


def package():
    f.validate()
    b.require(f.summarize() in (0, 1), 'invalid evaluation')
    manifest = b.read(f.STUDY / 'manifest.json'); index = {}; traces = []
    for scope in manifest['scopes']:
        for variant in ('default', 'cosine'):
            name = scope['id'] + '-' + variant
            result_path = f.STUDY / 'results' / (name + '.json')
            result = b.read(result_path); raw = b.ROOT / result['raw_directory']
            b.require(b.digest((raw / 'stdout.json').read_bytes()) == result['raw_report_sha256'], 'retained report drift')
            inventory = b.read(raw / 'evidence-inventory.json')
            b.require(b.identity(inventory) == result['evidence_inventory_identity'], 'retained inventory drift')
            report = b.read(raw / 'stdout.json'); run = pathlib.Path(report['run_directory'])
            for path, digest in inventory.items():
                b.require(b.digest((run / path).read_bytes()) == digest, 'retained artifact drift')
            recomputed = b.assess(report, scope, manifest, variant, f.STUDY)
            for field in ('mapping', 'pairs', 'canonical_pairs', 'metrics', 'units', 'functions'):
                b.require(recomputed[field] == result[field], 'raw/saved semantic drift: ' + field)
            b.require(f.correctness(report, result, manifest) == result['correctness'], 'raw/saved correctness drift')
            index[name] = {'result_sha256': b.digest(result_path.read_bytes()), 'raw_directory': result['raw_directory'],
                           'raw_report_sha256': result['raw_report_sha256'], 'run_directory': str(run),
                           'inventory_identity': result['evidence_inventory_identity'], 'inventory': inventory,
                           'completeness': {flag: report[flag] for flag in b.FLAGS}}
            if variant != 'default': continue
            mapped = {m['key']: m for m in result['mapping']}
            functions = []
            for record in report['functions']:
                m = mapped[record['key']]
                if m['test']: continue
                data = {'study_id': m['id'], 'key': record['key'], 'outcome': record['outcome'], 'reasons': record['reasons'],
                        'facts': record['facts'], 'projection_trace': record['projection_trace'],
                        'extraction_evidence': record['extraction_evidence'], 'document_manifest': record['document_manifest']}
                if record['outcome'] == 'admitted':
                    prefix = pathlib.Path(record['artifact_prefix'])
                    data['pdg'] = b.read(prefix.with_suffix('.pdg.json'))
                    data['cfg'] = b.read(prefix.with_suffix('.cfg.json'))
                functions.append(data)
            traces.append({'scope': scope, 'canonical_pairs': result['canonical_pairs'],
                           'pairs': [p for p in result['pairs'] if not p['first_test'] and not p['second_test']],
                           'functions': sorted(functions, key=lambda v: v['study_id'])})
    f.write(f.STUDY / 'results/evidence-index.json', index)
    f.write(f.STUDY / 'results/representative-traces.json', traces)

if __name__ == '__main__':
    package()
