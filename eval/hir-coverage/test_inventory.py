"""Independent eligibility bookkeeping checks, including fail-closed mapping."""
import copy
import hashlib
import unittest
import json
import tempfile
from pathlib import Path
import xxhash
import jsonschema
import reproduce
import assess
import summarize
import correspondence


class InventoryTests(unittest.TestCase):
    def setUp(self):
        source = 'fn f(x: i32) -> i32 { x as i32 }'
        self.selected = dict(package='p', version='1', path='src/lib.rs', start=0, end=len(source), source=source, name='f', sha256=hashlib.sha256(source.encode()).hexdigest())
        self.unit = dict(target=dict(package='p', name='p', manifest_dir='/p'), test=False)
        self.record = dict(outcome='excluded', reasons=['casts', 'conditional evidence'], key='f', compilation_unit=correspondence.context_key(self.unit), facts=dict(fn_span=dict(start=0, end=len(source)), source=dict(bytes=source, digest='sha256:'+self.selected['sha256']), source_diagnostics=[dict(capability='rust.integer-casts')], source_scan_complete=False, build=dict(frontend_options=dict(source_file='/p/src/lib.rs', source_identifier='f'))))
        self.report = dict(capture_complete=True, functions=[self.record], observed_units=[self.unit])

    def test_sealed_bundle_verification_rejects_tampering_and_missing_roots(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'unit.evidence.json'
            bundle = dict(schema='rstyle.shared-unit/5', namespace={'ordinal': 0}, roots=[0],
                          graphs=[dict(cfg={}, pdg={})])
            report = dict(functions=[dict(outcome='admitted', unit_evidence=dict(
                bundle=str(path), namespace=bundle['namespace'], function=0))])
            validators = {kind: jsonschema.Draft202012Validator({'type': 'object'})
                          for kind in ('cfg', 'pdg')}

            def seal():
                data = json.dumps(bundle).encode()
                path.write_bytes(data)
                path.with_suffix('.manifest.json').write_text(json.dumps(dict(
                    schema='rstyle.shared-unit.manifest/2', bytes=len(data),
                    checksum='xxh3-64:' + xxhash.xxh3_64_hexdigest(data))))

            seal()
            self.assertEqual(reproduce.verify_documents(report, validators), 2)
            path.write_bytes(path.read_bytes() + b' ')
            with self.assertRaisesRegex(ValueError, 'exact manifest'):
                reproduce.verify_documents(report, validators)
            bundle['roots'] = [None]
            seal()
            with self.assertRaisesRegex(ValueError, 'no graph root'):
                reproduce.verify_documents(report, validators)

    def test_all_reasons_and_source_blockers_survive(self):
        result = assess.assess([self.selected], {'p': self.report})
        f = result['functions'][0]
        self.assertEqual(set(f['reasons']), {'casts', 'conditional evidence'})
        self.assertEqual(f['source_blockers'][0]['capability'], 'rust.integer-casts')
        self.assertFalse(f['source_scan_complete'])

    def test_missing_ambiguous_and_failed_capture(self):
        for records, complete, expected in [([], True, 'unknown'), ([self.record, self.record], True, 'failed'), ([self.record], False, 'failed')]:
            result = assess.assess([self.selected], {'p': dict(self.report, capture_complete=complete, functions=records, capture_failures=['compiler failed'])})
            self.assertEqual(result['functions'][0]['outcome'], expected)

    def test_syntax_inventory_keeps_unreachable_blocker_combinations(self):
        blockers = assess.source_blockers('fn f() { return; loop { g(); m!(); } }')
        self.assertEqual({b['capability'] for b in blockers}, {'rust.loops', 'rust.call-summaries', 'rust.macros'})

    def test_measured_gain_counts_functions_once_and_preserves_unknowns(self):
        baseline = assess.assess([self.selected], {'p': self.report})
        baseline['stage'] = 'baseline'
        after = copy.deepcopy(baseline)
        after.update(stage='after', counts={'admitted': 1})
        after['functions'][0].update(outcome='admitted', reasons=[])
        result = summarize.summarize([baseline, after])
        self.assertEqual(len(result['slices'][1]['newly_admitted']), 1)
        self.assertEqual(len(result['hypothetical_source_syntax_bundles']['rust.integer-casts']), 1)
        changed = copy.deepcopy(after)
        changed['functions'][0]['sha256'] = 'edited'
        with self.assertRaises(ValueError):
            summarize.summarize([baseline, changed])

    def test_inventory_budget_does_not_hide_capture_or_evaluation_failures(self):
        report = dict(capture_complete=True, assessment_complete=True,
                      capture_failures=[], handoff_failures=[],
                      matching=dict(pairs=[], diagnostics=[
                          'pair enumeration requires 6 pairs, exceeds explicit resource limit 0']))
        reproduce.check_capture(report, 2)
        reproduce.check_capture(dict(report, matching=dict(
            pairs=[], diagnostics=[], incomplete=False)), 0)
        for changed, exit_code in [
            (dict(capture_complete=False), 2),
            (dict(assessment_complete=False), 2),
            (dict(handoff_failures=['missing manifest']), 2),
            (dict(capture_failures=['compiler failed']), 2),
            (dict(matching=dict(pairs=[{}], diagnostics=[])), 0),
            (dict(matching=dict(pairs=[], diagnostics=['arithmetic overflow'])), 2),
            (dict(matching=dict(pairs=[], diagnostics=['arithmetic overflow'], incomplete=False)), 0),
            ({}, 1),
        ]:
            with self.subTest(changed=changed, exit_code=exit_code):
                with self.assertRaises(ValueError):
                    reproduce.check_capture(dict(report, **changed), exit_code)


if __name__ == '__main__':
    unittest.main()
