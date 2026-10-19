# Reproduce the HIR coverage inventory

From the repository root:

```sh
uv run eval/hir-coverage/reproduce.py
```

Requires Git, rustup, network access to crates.io, and the pinned
`nightly-2026-09-30` compiler/components from `rust-toolchain.toml` on
`aarch64-unknown-linux-gnu` (commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`). The runner checks this context before
building; other hosts are explicitly unsupported for this inventory. `uv` supplies
Python >=3.12 and the script's pinned parser/schema dependencies.

The runner downloads and checks the three archives in `provenance.json`, verifies
that parsing reproduces every definition in `selection.json`, and installs the
small pinned dependency lockfiles in `locks/`. Source and manifests are unmodified.
Packages use temporary directories outside the repository to avoid Cargo's
enclosing-workspace discovery. Lockfiles are checked after capture as well.

Each invocation creates a fresh ignored `target/hir-coverage/run-*` directory.
It builds the actual driver and Cargo command, runs R0, the complete expanded profile, and Data-v3
with default features/debug settings, verifies capture/assessment completeness,
and checks actual admitted PDG/CFG documents inside sealed unit bundles against their exact manifests and
the unchanged pinned schemas. Inherited `RSTYLE_*` and build override variables
are cleared. Do not run competing workspace builds that replace these executables.

Generated outputs include:

- `run.json`: Git revision/working-tree state, compiler and selected profiles.
- `<profile>/<package>/`: command, stdout, stderr and actual capture targets,
  including paired documents and completion ledgers.
- `<profile>.json`: all 60 selected outcomes and all reasons/source blockers.
- `summary.json`: newly admitted/lost functions and remaining blocker combinations.
- `failure.txt`: an explicit runner error if reproduction did not complete.

Upload this generated directory as a CI artifact when diagnostics are useful;
remove it locally when finished. It is never an evidence cache or an extraction
input. Temporary downloaded sources are cleaned up; their exact contents remain
recreatable from the checked archives. No old run directory is discovered or reused.
Generated navigation paths into temporary sources expire after the run; frozen
package/path/span identities remain available in selection and assessments.

For a focused rerun, use `--profiles hir-data-v3`, `--profiles hir-expanded-v1`, or `--profiles r0`.
Its summary describes that subset, not measured gains for omitted slices. Inventory
commands use `--max-pairs 0`, never inspect predictions, and accept exit 2 only for
the explicit pair budget with complete capture/assessment. Failed or unknown
function assessments remain visible; they are not command failures or admissions.
Compare semantic assessments, not byte-identical logs or transient paths.

Run the small bookkeeping/failure tests with:

```sh
uv run --no-project --with tree-sitter==0.25.2 --with tree-sitter-rust==0.24.0 \
  --with jsonschema==4.25.1 --with xxhash==3.6.0 python -m unittest discover -s eval/hir-coverage -p 'test_*.py'
```

Independent extraction/dependence, adversarial verification and real Cargo tests
remain in the Rust suites. This inventory does not replace them, and no saved
capture outputs are required by those tests. The two historical quality studies
remain unchanged.

The intermediate stage table in REPORT.md is historical. To reproduce all eight
stages, use the lightweight runner at commit `e3261be7dc678351825f91301c1eab2c34e287e7`
in a separate checkout. For example:

```sh
git worktree add --detach /tmp/rstyle-coverage-stages e3261be7dc678351825f91301c1eab2c34e287e7
cd /tmp/rstyle-coverage-stages
uv run eval/hir-coverage/reproduce.py
```

Current reproduction compares R0, the complete 0.7.0 semantics, and Data-v3. Historical
stage identities remain in verification and independent Rust fixtures, without
requiring seven production extraction choices.
# Assessment version 2

Current reproduction uses exact source ownership and compilation contexts, rather
than interval containment. See [REPORT-v2.md](REPORT-v2.md) for corrected baseline
measurements; [REPORT.md](REPORT.md) retains historical conclusions. All frozen
selected definitions remain in the denominator. Missing capture is unknown, never
proof of cfg inactivity. Multiple contexts retain separate outcomes; missing or
duplicate identity evidence fails closed.

The 2026-10-19 refresh adds Data-v3 and current sealed unit-bundle verification.
After capture repairs and scalar OR patterns, Data-v3 3.1 completes with
9 admitted, 44 excluded and
7 unknown out of the unchanged 60 selected contexts. See the
[refresh report](REPORT-data-v3.md) and [continuation log](../../rust-feature-coverage.md).
