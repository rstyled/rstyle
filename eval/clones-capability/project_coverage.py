#!/usr/bin/env python3
"""Reduce an explicit project capture; sample agreement is not project accuracy."""
import argparse
from collections import Counter
import json
from pathlib import Path
import re

MAX_BYTES = 64 * 1024 * 1024
RULES = ('fn_length', 'fn_params', 'fn_spacing')
EXECUTIONS = {'completed', 'failed', 'budget_exhausted', 'not_requested'}
OUTCOMES = {'finding', 'evaluated_no_finding', 'not_evaluated'}
AVAILABILITY = {'known', 'unknown', 'unsupported', 'not_applicable'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def integer(value, name):
    require(type(value) is int and value >= 0, 'invalid nonnegative integer: ' + name)
    return value


def status(value, choices, name):
    require(value in choices, 'invalid ' + name)
    return value


def boolean(value, name):
    require(type(value) is bool, 'invalid boolean: ' + name)
    return value


def unique(rows, field, name):
    require(isinstance(rows, list), 'missing list: ' + name)
    result = {}
    for row in rows:
        require(isinstance(row, dict), 'invalid row: ' + name)
        key = row.get(field)
        require(isinstance(key, (str, int)) and type(key) is not bool,
                'invalid key: ' + name)
        require(key not in result, 'duplicate key: ' + name)
        result[key] = row
    return result


def relative_path(value, root, label=False):
    require(isinstance(value, str) and value != '', 'source path unavailable')
    path = Path(value)
    require('..' not in path.parts and '.' not in value.split('/'), 'source path traversal')
    if label:
        require(not path.is_absolute(), 'label path must be workspace-relative')
    if path.is_absolute():
        try:
            path = path.relative_to(root)
        except ValueError as error:
            raise ValueError('source path outside workspace') from error
    return path.as_posix()


def context(report, labels):
    scope = report['scope']
    require(scope.get('packages') == [labels['package']], 'package selection differs from labels')
    require(scope.get('all_targets') is False, 'study requires library-only selection')
    builds = scope['build_options']
    require(builds.get('source_rules') is True and builds.get('effect_density') is True,
            'study requires both consumers')
    require(builds.get('guard_clauses') is False, 'study excludes guard analysis')
    require(builds.get('mutation_effects') is False, 'study excludes scalar mutation analysis')
    require(builds.get('changed_files') is None, 'changed-file study is not full-project coverage')
    require(labels.get('target') == 'lib', 'study supports library targets only')
    features = labels.get('features')
    require(isinstance(features, list) and all(isinstance(x, str) for x in features),
            'invalid features')
    intended = report['intended_primary_units']
    require(isinstance(intended, list) and len(intended) == 1, 'ambiguous intended primary unit')
    unit = intended[0]
    require(unit.get('mode') == 'check' and unit.get('target', {}).get('kind') == ['lib'],
            'study requires check-mode library')
    require(unit.get('features') == features, 'features differ from labels')
    observed = report['observed_units']
    require(isinstance(observed, list) and len(observed) == 1, 'ambiguous observed unit')
    actual = observed[0]
    target = actual.get('target', {})
    require(actual.get('test') is False and target.get('kind') == ['lib'], 'wrong observed target')
    require(target.get('package_context', {}).get('name') == labels['package'], 'wrong package context')
    require(target.get('package') == unit.get('pkg_id') and
            target.get('name') == unit['target'].get('name') and
            target.get('source') == unit['target'].get('src_path'), 'observed/intended context mismatch')
    contexts = unique(report['unit_contexts'], 'id', 'unit contexts')
    require(len(contexts) == 1, 'ambiguous unit context')
    unit_id, association = next(iter(contexts.items()))
    integer(unit_id, 'unit id')
    require(integer(association.get('observed_unit'), 'observed unit') == 0, 'invalid unit reference')
    return unit_id, {'package': labels['package'], 'target': unit['target'],
                     'features': features, 'profile': unit.get('profile'),
                     'platform': unit.get('platform'), 'mode': unit['mode'],
                     'observed_target': target}


def roster(report, unit_id):
    observed = report['observed_units'][0]
    target = observed['target']
    compilation_unit = target['package'] + ':' + target['name'] + ':test=false'
    rows = unique(report['functions'], 'key', 'function roster')
    for key, row in rows.items():
        match = re.fullmatch(r'function:u([0-9]+):f([0-9]+)', key)
        require(match is not None and int(match[1]) == unit_id, 'function unit mismatch')
        require(isinstance(row.get('name'), str), 'function name unavailable')
        require(key == 'function:u' + str(unit_id) + ':f' + str(int(match[2])),
                'noncanonical function key')
        require(row.get('compilation_unit') == compilation_unit, 'compilation context mismatch')
        evidence = row['unit_evidence']
        require(integer(evidence.get('report_unit'), 'evidence unit') == unit_id and
                integer(evidence.get('function'), 'evidence function') == int(match[2]),
                'function evidence association mismatch')
        span = row['fn_span']
        require(integer(span.get('end'), 'span end') >= integer(span.get('start'), 'span start'),
                'invalid span')
    # A failed handoff may expose no roster despite a completed compiler capture.
    known = report['capture_complete'] and report['assessment_complete']
    return rows, len(rows) if known else None


def joined_rows(consumer, owners):
    rows = unique(consumer['functions'], 'key', 'consumer functions')
    require(set(rows) <= set(owners), 'consumer has extra owners')
    for key, row in rows.items():
        owner = owners[key]
        require(row.get('name') == owner['name'] and row.get('fn_span') == owner['fn_span'],
                'consumer owner identity mismatch')
        if 'source_digest' in row:
            require(row['source_digest'] == owner.get('source_digest'), 'consumer source mismatch')
    expected = consumer.get('expected_functions', consumer.get('scope', {}).get('expected_functions'))
    if expected is not None:
        require(integer(expected, 'expected functions') == len(owners), 'expected roster mismatch')
    return rows


def source_policy(row, rule):
    exclusion = row.get('source_exclusion') or {}
    suppression = row.get('source_suppression') or {}
    excluded_status = status(exclusion.get('status'),
                             {'included', 'excluded', 'not_applicable', 'unavailable'},
                             'source exclusion')
    excluded = excluded_status == 'excluded'
    suppression_status = suppression.get('status')
    if not excluded or suppression_status is not None:
        status(suppression_status, {'none', 'allowed', 'invalid', 'unavailable'}, 'source suppression')
    rules = suppression.get('rules', [])
    require(isinstance(rules, list) and all(isinstance(item, str) for item in rules),
            'invalid suppressed rule list')
    suppressed = suppression_status == 'allowed' and rule in rules
    unavailable = excluded_status == 'unavailable' or (not excluded and suppression_status == 'unavailable')
    invalid = suppression_status == 'invalid'
    return {'excluded': excluded, 'suppressed': suppressed,
            'source_policy_unavailable': unavailable, 'invalid_suppression': invalid}


def assessment(value):
    outcome = status(value.get('outcome'), OUTCOMES, 'outcome')
    availability = status(value.get('availability'), AVAILABILITY, 'availability')
    coverage = status(value.get('coverage'), {'complete', 'incomplete'}, 'coverage')
    integer(value.get('limit'), 'limit')
    if value.get('actual') is not None:
        integer(value['actual'], 'actual')
    if outcome != 'not_evaluated':
        disabled = value.get('rule_id') == 'fn_spacing' and outcome == 'evaluated_no_finding' and \
            availability == 'not_applicable' and value.get('actual') is None and value['limit'] == 0
        require(coverage == 'complete' and (availability == 'known' or disabled),
                'unproved evaluated outcome')
        if not disabled:
            integer(value.get('actual'), 'evaluated actual')
    return outcome, availability


def source_summary(consumer, owners):
    require(consumer.get('schema') == 'rstyle.consumer-analysis/1', 'wrong source schema')
    status(consumer.get('execution'), EXECUTIONS, 'source execution')
    rows = joined_rows(consumer, owners)
    counts = {rule: Counter() for rule in RULES}
    assessments = {}
    for key in owners:
        row = rows.get(key)
        if row is None:
            for count in counts.values():
                count['missing'] += 1
            continue
        execution = status(row.get('execution'), EXECUTIONS, 'function execution')
        items = unique(row['assessments'], 'rule_id', 'assessments')
        require(set(items) <= set(RULES) | {'invalid_suppression'}, 'unexpected source rule')
        for value in items.values():
            assessment(value)
        assessments[key] = items
        for rule in RULES:
            count = counts[rule]
            value = items.get(rule)
            if value is None:
                count['missing'] += 1
                continue
            outcome, availability = assessment(value)
            count[execution] += 1
            count[outcome] += 1
            count[availability] += 1
            policy = source_policy(row, rule)
            count.update(policy)
            count['disabled'] += availability == 'not_applicable' and outcome == 'evaluated_no_finding'
            count['known_evaluated'] += outcome != 'not_evaluated' and availability == 'known' and not policy['source_policy_unavailable'] and not policy['excluded']
    scope = consumer['scope']
    assessed = sum(row['execution'] != 'budget_exhausted' for row in rows.values())
    require(integer(scope['assessed_functions'], 'assessed functions') == assessed,
            'source assessed count mismatch')
    raw_findings = sum(1 for items in assessments.values() for value in items.values()
                       if value.get('outcome') == 'finding')
    require(integer(consumer['findings'], 'source findings') == raw_findings, 'source findings mismatch')
    evaluated = sum(value['outcome'] != 'not_evaluated' for items in assessments.values() for value in items.values())
    excluded_rules = sum(value['outcome'] == 'not_evaluated' and value['coverage'] == 'complete'
                         for items in assessments.values() for value in items.values())
    unavailable_rules = sum(value['outcome'] == 'not_evaluated' for items in assessments.values()
                            for value in items.values()) - excluded_rules
    for field, expected in (('evaluated_rules', evaluated), ('excluded_rules', excluded_rules),
                            ('unavailable_rules', unavailable_rules)):
        require(integer(scope[field], field) == expected, 'source aggregate mismatch: ' + field)
    visible = sum(value['outcome'] == 'finding' and
                  not source_policy(rows[key], value['rule_id'])['suppressed']
                  for key, items in assessments.items() for value in items.values())
    require(integer(consumer['visible_findings'], 'visible source findings') == visible and
            integer(consumer['suppressed_findings'], 'suppressed source findings') == raw_findings - visible,
            'source finding visibility mismatch')
    declared = status(scope.get('coverage'), {'complete', 'incomplete'}, 'source coverage')
    coverage = 'complete' if declared == 'complete' and len(rows) == len(owners) and not any(
        count['missing'] for count in counts.values()) else 'incomplete'
    return rows, assessments, {'execution': consumer['execution'], 'coverage': coverage,
                              'declared_coverage': declared,
                              'reasons': scope['reasons'], 'policy': consumer['effective_config'],
                              'missing_owner_rows': len(owners) - len(rows),
                              'rules': {rule: dict(sorted(count.items())) for rule, count in counts.items()}}


def density_lifecycle(row):
    evidence = row['evidence']
    execution = status(evidence.get('execution'), {'completed', 'budget_exhausted'}, 'effect execution')
    availability = status(evidence.get('availability', {}).get('status'), AVAILABILITY, 'effect availability')
    signature = status(evidence.get('signature', {}).get('status'), AVAILABILITY, 'effect signature')
    coverage = status(evidence.get('coverage'), {'complete', 'incomplete'}, 'effect coverage')
    statements = row['statement_inventory']
    statement_execution = status(statements.get('execution'), {'completed', 'budget_exhausted'}, 'statement execution')
    statement_status = status(statements.get('statements', {}).get('status'), AVAILABILITY, 'statement availability')
    if statement_status == 'known':
        integer(statements['statements'].get('value'), 'statement inventory')
    assessed = execution != 'budget_exhausted' and (row.get('density') is None or statement_execution != 'budget_exhausted')
    evaluated = assessed and (row.get('density') is None or row['density']['complete'])
    return {'effect_execution': execution, 'effect_availability': availability,
            'effect_signature': signature, 'effect_coverage': coverage,
            'statement_execution': statement_execution, 'statement_availability': statement_status,
            'assessed': assessed, 'evaluated': evaluated}


def density_summary(consumer, owners):
    require(consumer.get('schema') == 'rstyle.mutation-effects/4', 'wrong density schema')
    require(consumer.get('density_requested') is True, 'density not requested')
    require(consumer.get('scalar_mutations_requested') is False, 'study excludes scalar mutation analysis')
    status(consumer.get('execution'), EXECUTIONS, 'density execution')
    rows = joined_rows(consumer, owners)
    counts = Counter(missing=len(owners) - len(rows))
    lifecycle_counts = {}
    functions = []
    totals = Counter()
    for key, row in rows.items():
        lifecycle = density_lifecycle(row)
        for field, value in lifecycle.items():
            if type(value) is bool:
                totals[field] += value
            else:
                lifecycle_counts.setdefault(field, Counter())[value] += 1
        detail = {'key': key, 'name': row['name'], 'source_file': row.get('source_file'), **lifecycle,
                  'effect_reasons': row['evidence'].get('reasons', []),
                  'effect_availability_evidence': row['evidence']['availability'],
                  'effect_signature_evidence': row['evidence']['signature'],
                  'statement_inventory': row['statement_inventory']}
        functions.append(detail)
        value = row.get('density')
        if value is None:
            counts['not_requested'] += 1
            continue
        measurement = value['measurement']
        outcome = status(measurement.get('outcome'), OUTCOMES, 'density outcome')
        boolean(value.get('complete'), 'density complete')
        integer(measurement.get('limit'), 'density limit')
        if measurement.get('actual') is not None:
            integer(measurement['actual'], 'density actual')
        inventory = value['event_inventory']
        status(inventory.get('coverage'), {'complete', 'incomplete'}, 'event coverage')
        for count in inventory['counts'].values():
            integer(count, 'event count')
        complete_inventory = inventory.get('coverage') == 'complete'
        policy = source_policy(value, 'side_effect_density')
        known = value['complete'] and complete_inventory and not policy['source_policy_unavailable'] and not policy['excluded']
        if outcome != 'not_evaluated':
            require(complete_inventory, 'density evaluated with incomplete event inventory')
            statements = row['statement_inventory']
            require(statements.get('execution') == 'completed' and
                    statements.get('statements', {}).get('status') == 'known',
                    'density evaluated with unavailable statement inventory')
            source = measurement['source']
            require(integer(statements['statements'].get('value'), 'statement inventory') ==
                    source.get('statements'), 'density statement denominator mismatch')
            integer(source.get('statements'), 'density statements')
            integer(source.get('code_lines'), 'density lines')
            require(inventory['counts'].get('unknown', 0) == 0, 'density evaluated with unknown events')
        invalid = value.get('invalid_suppression') is not None
        require(invalid == (policy['invalid_suppression'] and not policy['excluded']),
                'density invalid suppression mismatch')
        expected_findings = int(outcome == 'finding') + int(invalid)
        expected_visible = int(outcome == 'finding' and not policy['excluded'] and not policy['suppressed']) + int(invalid)
        require(integer(value['findings'], 'density findings') == expected_findings and
                integer(value['visible_findings'], 'density visible findings') == expected_visible,
                'density function findings mismatch')
        totals['findings'] += expected_findings
        totals['visible_findings'] += expected_visible
        counts[outcome] += 1
        counts['known_evaluated'] += known and outcome != 'not_evaluated'
        counts['incomplete_event_inventory'] += not complete_inventory
        counts.update(policy)
        detail.update(outcome=outcome, complete=value['complete'], event_coverage=inventory['coverage'],
                      event_counts=inventory['counts'],
                      measurement=measurement, source_policy=policy,
                      unresolved_boundaries=inventory.get('unresolved_boundaries', []))
    for field, expected in (('assessed_functions', totals['assessed']),
                            ('evaluated_functions', totals['evaluated']),
                            ('unavailable_functions', len(rows) - totals['evaluated']),
                            ('findings', totals['findings'])):
        require(integer(consumer[field], field) == expected, 'density aggregate mismatch: ' + field)
    hidden = integer(consumer['review_hidden_findings'], 'density review-hidden findings')
    require(hidden <= totals['visible_findings'] and
            integer(consumer['visible_findings'], 'density visible findings') == totals['visible_findings'] - hidden,
            'density aggregate visibility mismatch')
    declared = status(consumer.get('coverage'), {'complete', 'incomplete'}, 'density coverage')
    coverage = 'complete' if declared == 'complete' and not counts['missing'] else 'incomplete'
    return {'execution': consumer['execution'], 'coverage': coverage, 'declared_coverage': declared,
            'reasons': consumer['reasons'], 'policy': consumer['density_config'],
            'counts': dict(sorted(counts.items())),
            'lifecycle': {field: dict(sorted(values.items())) for field, values in sorted(lifecycle_counts.items())},
            'aggregates': {field: consumer[field] for field in ('assessed_functions', 'evaluated_functions',
                           'unavailable_functions', 'findings', 'visible_findings', 'review_hidden_findings')},
            'functions': sorted(functions, key=lambda row: row['key'])}


def quality(labels, rows, assessments, root, unit_id, owners):
    index = {}
    for key, row in rows.items():
        path = row.get('source_file')
        if path is None:
            continue
        identity = (unit_id, row['name'], relative_path(path, root))
        index.setdefault(identity, []).append(key)
    results = []
    seen = set()
    for sample in labels['samples']:
        identity = (unit_id, sample['function'], relative_path(sample['source_file'], root, True))
        require(identity not in seen, 'duplicate sample identity')
        seen.add(identity)
        for rule, expected in sample['expected'].items():
            require(rule in RULES, 'unknown label rule')
            integer(expected.get('actual'), 'label actual')
            integer(expected.get('limit'), 'label limit')
            wanted = {**expected, 'outcome': expected['outcome'].replace('-', '_')}
            status(wanted['outcome'], OUTCOMES, 'label outcome')
            matches = index.get(identity, [])
            require(len(matches) <= 1, 'ambiguous sample owner identity')
            key = matches[0] if matches else None
            value = assessments.get(key, {}).get(rule)
            observed = None
            owner_seen = any(owner['name'] == identity[1] for owner in owners.values())
            result = 'unobserved' if key is None and not owner_seen else 'unavailable'
            if value is not None and value['outcome'] != 'not_evaluated':
                observed = {field: value.get(field) for field in ('actual', 'limit', 'outcome')}
                result = 'agreement' if observed == wanted else 'disagreement'
            evidence = None if value is None else {
                field: value.get(field) for field in ('outcome', 'coverage', 'availability', 'reasons')}
            results.append({'unit': unit_id, 'function': identity[1], 'source_file': identity[2],
                            'rule': rule, 'result': result, 'expected': wanted, 'observed': observed,
                            'assessment_evidence': evidence,
                            'function_execution': rows[key]['execution'] if key is not None else None})
    results.sort(key=lambda row: (row['source_file'], row['function'], row['rule']))
    return {'scope': 'independently specified source measurement sample',
            'counts': dict(sorted(Counter(row['result'] for row in results).items())),
            'comparisons': results,
            'limitation': 'Sample agreement does not establish project-wide precision or recall; density quality is unmeasured.'}


def summarize(report, labels, workspace_root):
    root = Path(workspace_root).absolute()
    require(labels.get('schema') == 'rstyle.clones-capability.project-study-cases/1', 'wrong labels schema')
    for field in ('capture_complete', 'assessment_complete', 'pair_enumeration_complete', 'pair_evaluation_complete'):
        boolean(report.get(field), field)
    unit_id, build = context(report, labels)
    owners, denominator = roster(report, unit_id)
    policy = labels.get('source_policy', {})
    for field in ('max_lines', 'max_params', 'min_statements'):
        integer(policy.get(field), 'source policy ' + field)
    boolean(policy.get('count_self'), 'count self')
    source = report['analysis']
    require(source.get('effective_config') == labels.get('source_policy'), 'source policy differs from labels')
    rows, assessments, sources = source_summary(source, owners)
    densities = density_summary(report['mutation_effects'], owners)
    return {'schema': 'rstyle.clones-capability.project-coverage/1',
            'context': build,
            'identity': {key: report.get(key) for key in ('toolchain', 'compiler_identity', 'cargo_identity',
                        'extraction_selection', 'extraction_policy', 'verification_policy')},
            'completion': {key: report[key] for key in ('capture_complete', 'assessment_complete',
                           'pair_enumeration_complete', 'pair_evaluation_complete')},
            'failures': {'capture': report.get('capture_failures', []), 'handoff': report.get('handoff_failures', [])},
            'roster': {'denominator': denominator, 'observed_rows': len(owners),
                       'status': 'known' if denominator is not None else 'unavailable'},
            'source': sources, 'density': densities,
            'quality': quality(labels, rows, assessments, root, unit_id, owners)}


def load(path):
    with Path(path).open('rb') as stream:
        data = stream.read(MAX_BYTES + 1)
    require(len(data) <= MAX_BYTES, 'input exceeds 64 MiB bound')
    def object_pairs(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate JSON object key: ' + key)
            result[key] = value
        return result

    return json.loads(data, object_pairs_hook=object_pairs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', required=True)
    parser.add_argument('--labels', required=True)
    parser.add_argument('--workspace-root', required=True)
    parser.add_argument('--output')
    args = parser.parse_args()
    try:
        result = summarize(load(args.report), load(args.labels), args.workspace_root)
        encoded = json.dumps(result, indent=2, sort_keys=True) + '\n'
        require(len(encoded.encode()) <= MAX_BYTES, 'output exceeds 64 MiB bound')
        if args.output:
            with Path(args.output).open('x') as stream:
                stream.write(encoded)
        else:
            print(encoded, end='')
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.exit(2, 'project coverage: ' + str(error) + '\n')


if __name__ == '__main__':
    main()
