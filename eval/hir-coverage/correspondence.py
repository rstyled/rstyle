"""Version 2 ownership checks against captured source and compilation contexts."""
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


def map_definition(selected, report, roster):
    decisions = []
    matched = []
    for context, manifest_dir in sorted(roster.items()):
        records = [record for record in report.get('functions', [])
                   if record.get('compilation_unit') == context
                   and candidate(selected, record, manifest_dir)]
        outcome = 'unknown'
        reasons = ['Exact selected definition absent from captured HIR; inactivity is unproved']
        if not report.get('capture_complete'):
            outcome = 'failed'
            reasons = report.get('capture_failures') or ['Capture unavailable']
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
    # A source-level admission requires every selected context to be admitted.
    for outcome in ('failed', 'excluded', 'unknown', 'admitted'):
        if any(decision['outcome'] == outcome for decision in decisions):
            reasons = sorted({reason for decision in decisions for reason in decision['reasons']})
            return outcome, reasons
    return 'failed', ['Unrecognized function outcome']
