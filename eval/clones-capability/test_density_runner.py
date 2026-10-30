"""Density runner policy/scope controls, without spawning Cargo."""
import json
from pathlib import Path
import tempfile
import unittest

from run_compare import roster
from run_density_compare import density_fixture, fixture_policy


class DensityRunnerTests(unittest.TestCase):
    def test_full_policy_matches_independent_labels_and_rejects_drift_or_unknown_depth(self):
        fixture = density_fixture()
        self.assertEqual(fixture.policy_reader(fixture.policy),
                         json.loads(fixture.expectations.read_text())['scope']['policy'])
        self.assertEqual(fixture.policy_reader(fixture.policy), fixture.new_policy_reader(fixture.new_policy))
        fixed = fixture.new_policy.read_text()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'policy.toml'
            for changed in (fixed.replace('max_density_milli = 500', 'max_density_milli = 501'),
                            fixed.replace('mutation = 1', 'mutation = true'),
                            fixed.replace('time = ["std::thread::yield_now"]', 'time = []'),
                            fixed.replace('"src/excluded.rs"', '"src/other.rs"'),
                            fixed.replace('min_lines = 0', 'min_lines = 0\ntransitive_depth = 3')):
                path.write_text(changed)
                with self.assertRaises(ValueError):
                    fixture_policy(path)

    def test_captured_density_roster_keeps_excluded_owner_and_requires_population_equality(self):
        facts = [{'name': 'density::single_write', 'path': 'src/lib.rs', 'line': 4},
                 {'name': 'density::excluded::write', 'path': 'src/excluded.rs', 'line': 1}]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'facts.json'
            path.write_text(json.dumps(facts))
            result = roster(path, {'summary': {'functions': 1}}, Path('/fixture'), 'excluded::write')
            self.assertEqual([entry['excluded'] for entry in result], [False, True])
            with self.assertRaises(ValueError):
                roster(path, {'summary': {'functions': 2}}, Path('/fixture'), 'excluded::write')

    def test_snapshots_retain_inputs_and_comparison_code(self):
        fixture = density_fixture()
        self.assertTrue(all(path.is_file() for path in fixture.inputs))
        self.assertIn(fixture.comparator, fixture.inputs)
        self.assertIn(fixture.expectations, fixture.inputs)
        self.assertEqual({path.name for path in fixture.inputs if path.suffix == '.rs'},
                         {'lib.rs', 'excluded.rs'})


if __name__ == '__main__':
    unittest.main()
