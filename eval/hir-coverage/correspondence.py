"""Version 3 ownership and independently inactive definition checks against captured source and compilation contexts."""
import hashlib
from pathlib import PurePosixPath


def context_key(unit):
    target = unit['target']
    return f"{target['package']}:{target['name']}:test={str(unit['test']).lower()}"


def contexts(report):
    """Use the observed selected-unit roster, never infer a build from a function."""
    units = report.get('observed_units', [])
    result = {}
    for unit in units:
        key = context_key(unit)
        if key in result:
            raise ValueError('Ambiguous compilation-context identity')
        result[key] = unit['target']['manifest_dir']
    return result


def candidate(selected, record, manifest_dir):
    facts = record['facts']
    file = facts['build']['frontend_options'].get('source_file')
    expected = PurePosixPath(manifest_dir) / selected['path']
    return (file is not None and PurePosixPath(file) == expected
            and facts['fn_span'] == dict(start=selected['start'], end=selected['end']))


def verify(selected, record):
    facts = record['facts']
    source = facts.get('source', {})
    if not isinstance(source.get('bytes'), str):
        return False
    expected = 'sha256:' + selected['sha256']
    actual = 'sha256:' + hashlib.sha256(source['bytes'].encode()).hexdigest()
    identifier = facts['build']['frontend_options'].get('source_identifier')
    return (source['bytes'] == selected['source']
            and source.get('digest') == expected == actual
            and identifier == selected['name'].removeprefix('r#'))


def inactive_candidates(selected, report, context, manifest_dir):
    expected = PurePosixPath(manifest_dir) / selected['path']
    return [entry for inventory in report.get('owned_source_inventory', [])
            if context_key(dict(target=inventory['compilation_unit'], test=inventory['test'])) == context
            for entry in inventory['source_functions']
            if entry.get('presence') == 'inactive_by_cfg'
            and PurePosixPath(entry['source_file']) == expected
            and entry.get('function_span') == dict(start=selected['start'], end=selected['end'])]


def verify_inactive(selected, entry):
    source = entry.get('source', '')
    span = entry.get('span', {})
    start = selected['start'] - span.get('start', selected['start'] + 1)
    end = selected['end'] - span.get('start', selected['start'] + 1)
    return (entry.get('inventory_status') == 'complete_for_source_definitions_outside_macro_tokens'
            and entry.get('active_functions') == []
            and entry.get('name', '').removeprefix('r#') == selected['name'].removeprefix('r#')
            and 0 <= start <= end <= len(source.encode())
            and source.encode()[start:end] == selected['source'].encode()
            and entry.get('source_digest') == 'sha256:' + hashlib.sha256(source.encode()).hexdigest()
            and entry.get('function_source_digest') == 'sha256:' + selected['sha256'])


def map_definition(selected, report, roster):
    decisions = []
    matched = []
    for context, manifest_dir in sorted(roster.items()):
        records = [record for record in report.get('functions', [])
                   if record.get('compilation_unit') == context
                   and candidate(selected, record, manifest_dir)]
        inactive = inactive_candidates(selected, report, context, manifest_dir)
        outcome = 'unknown'
        reasons = ['Exact selected definition absent from captured HIR; inactivity is unproved']
        if not report.get('capture_complete'):
            outcome = 'failed'
            reasons = report.get('capture_failures') or ['Capture unavailable']
        elif inactive and (records or len(inactive) != 1 or not verify_inactive(selected, inactive[0])):
            outcome, reasons = 'failed', ['Contradictory or invalid cfg-inactive source correspondence']
        elif inactive:
            outcome, reasons = 'inactive', ['Definition independently removed by compiler cfg settings']
        elif len(records) > 1:
            outcome, reasons = 'failed', ['Duplicate exact function correspondence in one compilation context']
        elif records and not verify(selected, records[0]):
            outcome, reasons = 'failed', ['Captured source identity or owned-function correspondence mismatch']
        elif records:
            outcome, reasons = records[0]['outcome'], records[0]['reasons']
            matched.extend(records)
        decision = dict(compilation_context=context, outcome=outcome, reasons=reasons,
                        captured_records=len(records),
                        active=len(records) == 1 and verify(selected, records[0]))
        if decision['active']:
            decision.update(function_key=records[0]['key'], build=records[0]['facts']['build'],
                            document_manifest=records[0].get('document_manifest'))
        decisions.append(decision)
    return decisions, matched


def aggregate(decisions):
    if not decisions:
        return 'failed', ['No observed selected compilation-context evidence']
    # Every active context must be admitted; independently inactive contexts do not execute.
    for outcome in ('failed', 'excluded', 'unknown', 'admitted', 'inactive'):
        if any(decision['outcome'] == outcome for decision in decisions):
            reasons = sorted({reason for decision in decisions for reason in decision['reasons']})
            return outcome, reasons
    return 'failed', ['Unrecognized function outcome']
