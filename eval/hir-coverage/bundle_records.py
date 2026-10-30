"""Read sealed bundle navigation without expanding HIR or source-option tables.

The Cargo command owns strict evidence validation. This reader independently
checks transport, graph schemas and the exact source facts used by the frozen
inventory. It is deliberately not a second implementation of UnitStore import.
"""
import json
from pathlib import Path

import xxhash


def index(table, value, label):
    if type(value) is not int or not 0 <= value < len(table):
        raise ValueError(f"Invalid {label} reference")
    return table[value]


def load(path):
    data = path.read_bytes()
    manifest = json.loads(path.with_suffix('.manifest.json').read_text())
    if (manifest['schema'] != 'rstyle.shared-unit.manifest/2'
            or manifest['bytes'] != len(data)
            or manifest['checksum'] != 'xxh3-64:' + xxhash.xxh3_64_hexdigest(data)):
        raise ValueError('Serialized unit does not match its exact manifest')
    bundle = json.loads(data)
    if bundle.get('schema') not in ('rstyle.shared-unit/4', 'rstyle.shared-unit/5', 'rstyle.shared-unit/6'):
        raise ValueError('Unsupported shared unit schema')
    return bundle


def source_facts(bundle, function, sources):
    reference = function['source']
    snapshot = reference['snapshot']
    text = index(bundle['sources'], snapshot, 'source snapshot')
    if not isinstance(text, str):
        raise ValueError('Invalid source snapshot')
    # Encode each immutable file once, rather than once per captured function.
    if snapshot not in sources:
        sources[snapshot] = text.encode('utf-8')
    data = sources[snapshot]
    span = reference['range']
    start, end = span['start'], span['end']
    if (type(start) is not int or type(end) is not int
            or not 0 <= start <= end <= len(data)):
        raise ValueError('Invalid source byte range')
    try:
        text = data[start:end].decode('utf-8')
    except UnicodeDecodeError as error:
        raise ValueError('Invalid UTF-8 source byte range') from error
    return dict(bytes=text, digest=reference['digest'])


def project(bundle, record, sources):
    """Keep only correspondence inputs; full evidence stays at unit_evidence."""
    evidence = record['unit_evidence']
    function = index(bundle['functions'], evidence['function'], 'function')
    for key, expected in (('name', function['name']), ('fn_span', function['fn_span']),
                          ('source_digest', function['source']['digest'])):
        if record[key] != expected:
            raise ValueError(f'Compact {key} disagrees with referenced function')
    build = function['build']
    options = build['frontend_options']
    return dict(fn_span=function['fn_span'], source=source_facts(bundle, function, sources),
                source_diagnostics=function['source_diagnostics'],
                source_scan_complete=function['source_scan_complete'],
                build=dict(frontend_options={key: options.get(key)
                                             for key in ('source_file', 'source_identifier')},
                           **{key: value for key, value in build.items()
                              if key != 'frontend_options'}))


def verify_documents(report, validators):
    """Consume one bundle at a time; attach narrow facts only for compact reports."""
    compact = report.get('report_detail') == 'compact'
    grouped = {}
    for record in report['functions']:
        if compact or record['outcome'] == 'admitted':
            path = Path(record['unit_evidence']['bundle'])
            grouped.setdefault(path, []).append(record)
    checked = 0
    for path, records in grouped.items():
        bundle = load(path)
        sources = {}
        validated_roots = set()
        for record in records:
            evidence = record['unit_evidence']
            if bundle['namespace'] != evidence['namespace']:
                raise ValueError('Unit evidence namespace mismatch')
            if compact:
                if 'facts' in record:
                    raise ValueError('Compact report unexpectedly embeds full facts')
                record['facts'] = project(bundle, record, sources)
            if record['outcome'] != 'admitted':
                continue
            root = index(bundle['roots'], evidence['function'], 'function root')
            if root is None:
                raise ValueError('Admitted function has no graph root')
            graph = index(bundle['graphs'], root, 'graph')
            if root not in validated_roots:
                for kind in ('cfg', 'pdg'):
                    validators[kind].validate(graph[kind])
                validated_roots.add(root)
            checked += 2
    return checked
