#!/usr/bin/env python3
"""Package selected explanatory evidence from preserved, validated study runs."""
import pathlib
import ccgraph_study as s


def evidence():
    s.validate_freeze()
    manifest = s.read(s.STUDY / 'manifest.json')
    index = {}; traces = []
    selections = {
        'controlled-development': [('empty_base', 'empty_semantic')],
        'controlled-heldout': [('shadow_base', 'shadow_rename'), ('short_base', 'short_semantic'), ('short_base', 'merge_base')],
    }
    for scope in manifest['scopes']:
        for variant in ('default', 'cosine'):
            name = scope['id'] + '-' + variant
            result_path = s.STUDY / 'results' / (name + '.json')
            result = s.read(result_path)
            s.validate_result(result, scope, manifest, variant)
            raw = s.ROOT / result['raw_directory']
            require_digest = s.digest((raw / 'stdout.json').read_bytes())
            s.require(require_digest == result['raw_report_sha256'], 'retained raw report drift')
            inventory = s.read(raw / 'evidence-inventory.json')
            s.require(s.identity(inventory) == result['evidence_inventory_identity'], 'retained inventory drift')
            report = s.read(raw / 'stdout.json')
            index[name] = {'result_sha256': s.digest(result_path.read_bytes()), 'raw_directory': result['raw_directory'],
                           'raw_report_sha256': require_digest, 'run_directory': report['run_directory'],
                           'inventory_identity': s.identity(inventory), 'inventory': inventory,
                           'producer_build_revisions': sorted({f['facts']['build']['producer_revision']['value'] for f in report['functions']}),
                           'report_identity': {k: report[k] for k in ('toolchain', 'compiler_identity', 'cargo_identity', 'contract_revision', 'extraction_policy', 'schema_versions', 'scope')}}
            if variant != 'default': continue
            for modules in selections.get(scope['id'], []):
                pairs = [p for p in result['pairs'] if not p['first_test'] and not p['second_test']
                         and {p['study_first'].split(':')[-1], p['study_second'].split(':')[-1]} == set(modules)]
                s.require(len(pairs) == 1, 'representative trace mapping ambiguity')
                pair = pairs[0]; keys = {pair['first'], pair['second']}
                functions = []
                for f in report['functions']:
                    if f['key'] not in keys: continue
                    functions.append({k: f[k] for k in ('key', 'outcome', 'reasons', 'projection_trace', 'extraction_evidence', 'document_manifest')} |
                                     {'source': f['facts']['source'], 'fn_span': f['facts']['fn_span'], 'build': f['facts']['build']})
                label = next(r for r in result['canonical_pairs'] if set(r['detector_keys']) == keys)
                traces.append({'scope': scope['id'], 'label': label, 'pair': pair, 'functions': sorted(functions, key=lambda f: f['key'])})
    s.write(s.STUDY / 'results/evidence-index.json', index)
    s.write(s.STUDY / 'results/representative-traces.json', traces)

if __name__ == '__main__':
    evidence()
