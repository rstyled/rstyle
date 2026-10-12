"""Independent eligibility bookkeeping checks, including fail-closed mapping."""
import copy
import hashlib
import unittest
import gzip
import tempfile
from pathlib import Path
import retain
import assess
import summarize


class InventoryTests(unittest.TestCase):
    def setUp(self):
        source = 'fn f(x: i32) -> i32 { x as i32 }'
        self.selected = dict(package='p', version='1', path='src/lib.rs', start=0, end=len(source), source=source, name='f', sha256=hashlib.sha256(source.encode()).hexdigest())
        self.record = dict(outcome='excluded', reasons=['casts', 'conditional evidence'], key='f', facts=dict(fn_span=dict(start=0, end=len(source)), source=dict(digest='sha256:'+self.selected['sha256']), source_diagnostics=[dict(capability='rust.integer-casts')], source_scan_complete=False, build=dict(frontend_options=dict(source_file='/p/src/lib.rs'))))
        self.report = dict(capture_complete=True, functions=[self.record])

    def test_all_reasons_and_source_blockers_survive(self):
        result = assess.assess([self.selected], {'p': self.report})
        f = result['functions'][0]
        self.assertEqual(f['reasons'], ['casts', 'conditional evidence'])
        self.assertEqual(f['source_blockers'][0]['capability'], 'rust.integer-casts')
        self.assertFalse(f['source_scan_complete'])

    def test_missing_ambiguous_and_failed_capture(self):
        for records, complete, expected in [([], True, 'unknown'), ([self.record, self.record], True, 'failed'), ([self.record], False, 'failed')]:
            result = assess.assess([self.selected], {'p': dict(capture_complete=complete, functions=records, capture_failures=['compiler failed'])})
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

    def test_retained_bytes_and_digest_are_exact(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)/'document.json'
            destination = Path(directory)/'document.json.gz'
            data = b'{"original":"\\n"}\r\n'
            source.write_bytes(data)
            record = retain.retain(source, destination)
            self.assertEqual(gzip.decompress(destination.read_bytes()), data)
            self.assertEqual(record['original_sha256'], hashlib.sha256(data).hexdigest())


if __name__ == '__main__':
    unittest.main()
