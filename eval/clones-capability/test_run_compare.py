"""Lightweight runner checks; no actual Cargo capture/build is executed."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from run_compare import commands, controlled_environment, roster


class RunnerTests(unittest.TestCase):
    def test_commands_share_explicit_source_scope_and_config(self):
        old, new = commands(Path('/bin/tool'), Path('/fixture/Cargo.toml'), Path('/fixture/policy.toml'))
        for command in (old, new):
            self.assertEqual(command[command.index('--manifest-path') + 1], '/fixture/Cargo.toml')
            self.assertEqual(command[command.index('--package') + 1], 'clones-capability-acceptance')
            self.assertNotIn('--all-targets', command)
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
            path.write_text(json.dumps([{'name': 'sample::real', 'path': '/real/lib.rs', 'line': 33}]))
            self.assertEqual(roster(path, {'summary': {'functions': 1}}),
                             [{'name': 'sample::real', 'source_file': '/real/lib.rs', 'line': 33}])
            with self.assertRaises(ValueError):
                roster(path, {'summary': {'functions': 2}})
            path.write_text('[]')
            with self.assertRaises(ValueError):
                roster(path, {'summary': {'functions': 0}})


if __name__ == '__main__':
    unittest.main()
