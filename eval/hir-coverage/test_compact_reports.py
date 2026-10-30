"""Independent source expectations and full/compact inventory parity."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import jsonschema
import xxhash

import assess
import bundle_records
import summarize
from test_correspondence import inactive_inventory, selected, unit


class CompactReportTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.path = Path(self.temporary.name) / 'unit.evidence.json'
        self.namespace = dict(process=1, epoch_nanos=2, ordinal=0)
        first = selected('fn f() { let café = 1; }', start=3)
        second = selected('fn g() { unsupported!(); }', start=100, name='g')
        missing = selected('fn missing() {}', start=200, name='missing')
        inactive = selected('fn off() {}', start=12, name='off')
        self.selection = [first, second, missing, inactive]
        self.full = dict(report_detail='full', capture_complete=True,
                         observed_units=[unit()], functions=[],
                         owned_source_inventory=[inactive_inventory(inactive)])
        self.compact = dict(self.full, report_detail='compact', functions=[])
        self.bundle = dict(schema='rstyle.shared-unit/5', namespace=self.namespace,
                           sources=['π ' + first['source'], second['source']],
                           functions=[], roots=[0, None], graphs=[dict(
                               cfg=dict(valid=True), pdg=dict(valid=True))])
        for i, definition in enumerate((first, second)):
            source = dict(bytes=definition['source'], digest='sha256:' + definition['sha256'])
            facts = dict(name=definition['name'],
                         fn_span=dict(start=definition['start'], end=definition['end']),
                         source=source, source_scan_complete=i == 0,
                         source_diagnostics=[] if i == 0 else [dict(capability='rust.macros')],
                         build=dict(rustc_commit=dict(status='known', value='compiler'),
                                    frontend_options=dict(source_file='/p/src/lib.rs',
                                                          source_identifier=definition['name'],
                                                          census='FULL_EVIDENCE_ONLY')))
            evidence = dict(bundle=str(self.path), namespace=self.namespace, function=i)
            common = dict(key=f'function:u0:f{i}', compilation_unit='p:p:test=false',
                          outcome='admitted' if i == 0 else 'excluded',
                          reasons=[] if i == 0 else ['Unsupported macro'], unit_evidence=evidence)
            self.full['functions'].append(dict(common, facts=facts))
            self.compact['functions'].append(dict(common, name=facts['name'],
                                                 fn_span=facts['fn_span'],
                                                 source_digest=source['digest']))
            start = 3 if i == 0 else 0
            self.bundle['functions'].append(dict(facts, source=dict(
                snapshot=i, range=dict(start=start, end=start + len(definition['source'].encode())),
                digest=source['digest'])))
        schema = dict(type='object', required=['valid'], properties=dict(valid=dict(const=True)))
        self.validators = {kind: jsonschema.Draft202012Validator(schema) for kind in ('cfg', 'pdg')}
        self.seal()

    def seal(self):
        data = json.dumps(self.bundle).encode()
        self.path.write_bytes(data)
        self.path.with_suffix('.manifest.json').write_text(json.dumps(dict(
            schema='rstyle.shared-unit.manifest/2', bytes=len(data),
            checksum='xxh3-64:' + xxhash.xxh3_64_hexdigest(data))))

    def resolve(self, report=None):
        report = copy.deepcopy(self.compact if report is None else report)
        checked = bundle_records.verify_documents(report, self.validators)
        return report, checked

    def test_full_compact_parity_keeps_all_outcomes_denominators_and_navigation(self):
        full, checked_full = self.resolve(self.full)
        compact, checked_compact = self.resolve()
        expected = assess.assess(self.selection, {'p': full})
        actual = assess.assess(self.selection, {'p': compact})
        self.assertEqual(actual, expected)
        self.assertEqual((checked_full, checked_compact), (2, 2))
        self.assertEqual(actual['counts'], dict(admitted=1, excluded=1, unknown=1, inactive=1))
        self.assertEqual(actual['denominators']['all_frozen_selected'], 4)
        self.assertEqual(actual['denominators']['selected_function_contexts'], 4)
        self.assertEqual(actual['denominators']['active_function_contexts'], 2)
        self.assertEqual(actual['denominators']['proven_inactive_function_contexts'], 1)
        self.assertEqual(actual['functions'][0]['unit_evidence'], self.full['functions'][0]['unit_evidence'])
        self.assertEqual(actual['functions'][1]['frontend_diagnostics'], [dict(capability='rust.macros')])
        self.assertNotIn('FULL_EVIDENCE_ONLY', json.dumps(actual))
        self.assertNotIn('frontend_options', json.dumps(actual))
        self.assertEqual(compact['functions'][0]['facts']['source']['bytes'], self.selection[0]['source'])
        actual['stage'] = expected['stage'] = 'current'
        self.assertEqual(summarize.summarize([actual]), summarize.summarize([expected]))

    def test_failed_capture_and_missing_contexts_keep_their_distinct_outcomes(self):
        for changes in [dict(capture_complete=False, capture_failures=['compiler failed']),
                        dict(observed_units=[])]:
            full, _ = self.resolve(dict(self.full, **changes))
            compact, _ = self.resolve(dict(self.compact, **changes))
            expected = assess.assess(self.selection, {'p': full})
            actual = assess.assess(self.selection, {'p': compact})
            self.assertEqual(actual, expected)
            self.assertEqual(actual['counts'], dict(failed=4))

    def test_missing_reference_and_missing_bundle_never_become_unknown(self):
        report = copy.deepcopy(self.compact)
        del report['functions'][1]['unit_evidence']
        with self.assertRaises(KeyError):
            self.resolve(report)
        self.path.unlink()
        with self.assertRaises(FileNotFoundError):
            self.resolve()

    def test_wrong_namespace_function_reference_and_navigation_metadata_fail(self):
        for field, value in [('namespace', {}), ('function', -1), ('function', True),
                             ('function', 20)]:
            report = copy.deepcopy(self.compact)
            report['functions'][1]['unit_evidence'][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.resolve(report)
        for field, value in [('name', 'other'), ('fn_span', {}), ('source_digest', 'bad')]:
            report = copy.deepcopy(self.compact)
            report['functions'][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.resolve(report)

    def test_corrupt_manifest_bundle_schema_and_graph_schema_fail(self):
        self.path.write_bytes(self.path.read_bytes() + b' ')
        with self.assertRaisesRegex(ValueError, 'exact manifest'):
            self.resolve()
        self.bundle['schema'] = 'rstyle.shared-unit/unknown'
        self.seal()
        with self.assertRaisesRegex(ValueError, 'Unsupported shared unit schema'):
            self.resolve()
        self.bundle['schema'] = 'rstyle.shared-unit/5'
        self.bundle['graphs'][0]['pdg'] = {}
        self.seal()
        with self.assertRaises(jsonschema.ValidationError):
            self.resolve()

    def test_flat_body_bundle_version_preserves_narrow_navigation(self):
        self.bundle['schema'] = 'rstyle.shared-unit/6'
        # This reader needs source/graph navigation, not a second HIR decoder.
        self.bundle['functions'][0]['body'] = [{'node': {}, 'children': 0}]
        self.seal()
        self.resolve()

    def test_source_ranges_fail_closed_and_wrong_source_fails_correspondence(self):
        reference = self.bundle['functions'][0]['source']
        original = copy.deepcopy(reference)
        for changes in [dict(snapshot=-1), dict(snapshot=True), dict(snapshot=2),
                        dict(range=dict(start=-1, end=3)), dict(range=dict(start=0, end=999)),
                        dict(range=dict(start=1, end=3))]:
            self.bundle['functions'][0]['source'] = dict(original, **changes)
            self.seal()
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                self.resolve()
        # Same length and independently resealed transport cannot prove source identity.
        self.bundle['functions'][0]['source'] = original
        self.bundle['sources'][0] = self.bundle['sources'][0].replace('café', 'cafè')
        self.seal()
        report, _ = self.resolve()
        result = assess.assess(self.selection, {'p': report})
        self.assertEqual(result['functions'][0]['outcome'], 'failed')

    def test_one_bundle_load_and_one_utf8_encoding_per_snapshot(self):
        class CountedText(str):
            calls = 0

            def encode(self, *args, **kwargs):
                self.calls += 1
                return super().encode(*args, **kwargs)

        text = CountedText(self.bundle['sources'][0])
        self.bundle['sources'][0] = text
        # A repeated observation must remain visible, without repeated file work.
        report = copy.deepcopy(self.compact)
        report['functions'].append(copy.deepcopy(report['functions'][0]))
        with patch('bundle_records.load', return_value=self.bundle) as load:
            resolved, _ = self.resolve(report)
        load.assert_called_once_with(self.path)
        self.assertEqual(text.calls, 1)
        result = assess.assess(self.selection, {'p': resolved})
        self.assertEqual(result['functions'][0]['outcome'], 'failed')
        self.assertEqual(result['denominators']['captured_records'], 3)


if __name__ == '__main__':
    unittest.main()
