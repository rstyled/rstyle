"""Lightweight runner checks; no actual Cargo capture/build is executed."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from run_compare import FIXTURE_EXCLUDES, commands, controlled_environment, fixture_inputs, fixture_policy, roster


class RunnerTests(unittest.TestCase):
    def test_commands_share_explicit_source_scope_and_config(self):
        old, new = commands(Path('/bin/tool'), Path('/fixture/Cargo.toml'), Path('/fixture/policy.toml'))
        for command in (old, new):
            self.assertEqual(command[command.index('--manifest-path') + 1], '/fixture/Cargo.toml')
            self.assertEqual(command[command.index('--package') + 1], 'clones-capability-acceptance')
            self.assertNotIn('--all-targets', command)
        self.assertEqual(old[old.index('--style-engine') + 1], 'legacy')
        self.assertNotIn('--style-engine', new)
        self.assertEqual(old[-2:], ['--config', '/fixture/policy.toml'])
        self.assertIn('--locked', new)
        self.assertIn('--source-rules', new)
        self.assertEqual(new[-2:], ['--analysis-config', '/fixture/policy.toml'])
        self.assertIn('hir-data-v3', new)

    def test_inherited_capture_and_compiler_modes_are_removed(self):
        inherited = {'PATH': '/bin', 'RSTYLE_PDG': '1', 'RSTYLE_HIR_DOCUMENT_DIR': 'stale',
                     'RUSTFLAGS': 'bad', 'CARGO_BUILD_TARGET': 'other', 'CARGO_PROFILE_DEV_DEBUG': '0'}
        with patch.dict(os.environ, inherited, clear=True):
            env = controlled_environment(Path('/fresh'), 'pinned', Path('/driver'))
        self.assertNotIn('RSTYLE_PDG', env)
        self.assertNotIn('RSTYLE_HIR_DOCUMENT_DIR', env)
        self.assertNotIn('RUSTFLAGS', env)
        self.assertNotIn('CARGO_BUILD_TARGET', env)
        self.assertNotIn('CARGO_PROFILE_DEV_DEBUG', env)
        self.assertEqual(env['CARGO_TARGET_DIR'], '/fresh')
        self.assertEqual(env['RUSTUP_TOOLCHAIN'], 'pinned')

    def test_roster_comes_from_captured_fact_locations(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'facts.json'
            facts = [{'name': 'sample::real', 'path': '/real/src/lib.rs', 'line': 33},
                     {'name': 'sample::excluded::over_both_limits',
                      'path': '/real/src/excluded.rs', 'line': 3}]
            path.write_text(json.dumps(facts))
            root = Path('/real')
            result = roster(path, {'summary': {'functions': 1}}, root)
            self.assertEqual(result, [
                {'name': 'sample::real', 'source_file': '/real/src/lib.rs', 'line': 33, 'excluded': False},
                {'name': 'sample::excluded::over_both_limits',
                 'source_file': '/real/src/excluded.rs', 'line': 3, 'excluded': True}])
            with self.assertRaises(ValueError):
                roster(path, {'summary': {'functions': 2}}, root)
            for incomplete in ([], facts[:1], facts[1:], facts + [facts[0]], facts + [facts[1]]):
                path.write_text(json.dumps(incomplete))
                with self.assertRaises(ValueError):
                    roster(path, {'summary': {'functions': 1}}, root)

    def test_only_exact_original_excluded_path_is_declared(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'facts.json'
            facts = [{'name': 'sample::excluded::over_both_limits',
                      'path': '/real/src/excluded.rs', 'line': 3},
                     {'name': 'sample::external', 'path': '/outside/src/excluded.rs', 'line': 3}]
            path.write_text(json.dumps(facts))
            result = roster(path, {'summary': {'functions': 1}}, Path('/real'))
            self.assertFalse(result[1]['excluded'])

    def test_relative_original_paths_and_unprefixed_names_are_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'facts.json'
            facts = [{'name': 'excluded::over_both_limits',
                      'path': 'src/excluded.rs', 'line': 3},
                     {'name': 'source_rules::length_at_limit', 'path': 'src/lib.rs', 'line': 6}]
            path.write_text(json.dumps(facts))
            result = roster(path, {'summary': {'functions': 1}}, Path('/original/removed'))
            self.assertTrue(result[0]['excluded'])
            self.assertFalse(result[1]['excluded'])
            self.assertEqual(result[0]['source_file'], 'src/excluded.rs')
            self.assertEqual(result[0]['name'], 'excluded::over_both_limits')

    def test_snapshots_include_every_fixture_module_and_fixed_policy(self):
        directory = Path(__file__).resolve().parent
        manifest = directory / 'corpus/Cargo.toml'
        inputs = fixture_inputs(manifest, directory / 'analysis.toml', manifest.with_name('Cargo.lock'))
        modules = {path.name for path in inputs if path.suffix == '.rs'}
        self.assertEqual(modules, {'lib.rs', 'excluded.rs', 'spacing.rs'})
        self.assertTrue(all(path.is_file() for path in inputs))
        self.assertIn(directory / 'expectations.json', inputs)
        self.assertIn(directory / 'analysis.toml', inputs)

    def test_fixture_policy_rejects_exclusion_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            policy = Path(directory) / 'analysis.toml'
            fixed = Path(__file__).with_name('analysis.toml').read_text()
            policy.write_text(fixed)
            self.assertEqual(fixture_policy(policy)['exclude'], FIXTURE_EXCLUDES)
            self.assertEqual(fixture_policy(policy)['min_statements'], 5)
            for changed in (fixed.replace('"src/excluded.rs"', '"src/other.rs"'),
                            fixed.replace('"target/**", ', ''),
                            fixed.replace('max_lines = 5', 'max_lines = 6'),
                            fixed.replace('min_statements = 5', 'min_statements = 4'),
                            fixed.replace('[fn_spacing]\nmin_statements = 5\n', '')):
                policy.write_text(changed)
                with self.assertRaises(ValueError):
                    fixture_policy(policy)


if __name__ == '__main__':
    unittest.main()
