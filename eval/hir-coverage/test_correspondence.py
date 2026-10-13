"""Independent source/context mapping fixtures for assessment version 2."""
import copy
import hashlib
import unittest

import assess
import correspondence


def selected(source, start=0, name='f'):
    return dict(package='p', version='1', path='src/lib.rs', start=start,
                end=start + len(source.encode()), source=source, name=name,
                sha256=hashlib.sha256(source.encode()).hexdigest())


def unit(test=False):
    return dict(target=dict(package='p', name='p', manifest_dir='/p'), test=test)


def record(definition, test=False):
    return dict(key=f"{definition['start']}:{test}", outcome='admitted', reasons=[],
                compilation_unit=correspondence.context_key(unit(test)),
                facts=dict(fn_span=dict(start=definition['start'], end=definition['end']),
                           source=dict(bytes=definition['source'], digest='sha256:' + definition['sha256']),
                           build=dict(frontend_options=dict(source_file='/p/src/lib.rs',
                                                            source_identifier=definition['name'])),
                           source_diagnostics=[], source_scan_complete=True))


def report(records, units=None):
    return dict(capture_complete=True, functions=records,
                observed_units=units if units is not None else [unit()])


class CorrespondenceTests(unittest.TestCase):
    def assess(self, definitions, records, units=None):
        return assess.assess(definitions, {'p': report(records, units)})

    def test_nested_capture_cannot_admit_missing_outer(self):
        outer = selected('fn f() { fn g() {} }')
        inner = selected('fn g() {}', start=9, name='g')
        result = self.assess([outer, inner], [record(inner)])
        self.assertEqual(result['counts'], {'unknown': 1, 'admitted': 1})
        self.assertEqual(result['denominators']['all_frozen_selected'], 2)
        self.assertEqual(result['denominators']['active_function_contexts'], 1)
        self.assertEqual(result['denominators']['proven_inactive_function_contexts'], 0)

    def test_same_named_conditional_alternatives_use_exact_offsets(self):
        first = selected('fn f() {}', start=10)
        second = selected('fn f() {}', start=30)
        result = self.assess([first, second], [record(second)])
        self.assertEqual([f['outcome'] for f in result['functions']], ['unknown', 'admitted'])

    def test_original_bom_crlf_and_utf8_offsets(self):
        definition = selected('fn f() {\r\n let café = 1;\r\n}', start=3)
        result = self.assess([definition], [record(definition)])
        self.assertEqual(result['counts'], {'admitted': 1})
        wrong = record(definition)
        wrong['facts']['fn_span']['start'] = 0
        self.assertEqual(self.assess([definition], [wrong])['counts'], {'unknown': 1})

    def test_multiple_contexts_are_assessed_separately(self):
        definition = selected('fn f() {}')
        result = self.assess([definition], [record(definition), record(definition, True)],
                             [unit(), unit(True)])
        self.assertEqual(result['counts'], {'admitted': 1})
        self.assertEqual(result['context_counts'], {'admitted': 2})
        missing = self.assess([definition], [record(definition)], [unit(), unit(True)])
        self.assertEqual(missing['counts'], {'unknown': 1})
        self.assertEqual(missing['context_counts'], {'admitted': 1, 'unknown': 1})

    def test_duplicates_corruption_and_wrong_package_fail_closed(self):
        definition = selected('fn f() {}')
        captured = record(definition)
        self.assertEqual(self.assess([definition], [captured, captured])['counts'], {'failed': 1})
        for field, value in [('bytes', 'fn f() { }'), ('digest', 'sha256:bad')]:
            wrong = copy.deepcopy(captured)
            wrong['facts']['source'][field] = value
            self.assertEqual(self.assess([definition], [wrong])['counts'], {'failed': 1})
        missing = copy.deepcopy(captured)
        del missing['facts']['source']['bytes']
        self.assertEqual(self.assess([definition], [missing])['counts'], {'failed': 1})
        wrong = copy.deepcopy(captured)
        wrong['facts']['build']['frontend_options']['source_file'] = '/other/src/lib.rs'
        self.assertEqual(self.assess([definition], [wrong])['counts'], {'unknown': 1})
        with self.assertRaises(ValueError):
            self.assess([definition, definition], [captured])
        with self.assertRaises(ValueError):
            self.assess([definition], [captured], [unit(), unit()])

    def test_missing_context_roster_is_failed_evidence(self):
        definition = selected('fn f() {}')
        self.assertEqual(self.assess([definition], [record(definition)], [])['counts'], {'failed': 1})


if __name__ == '__main__':
    unittest.main()
