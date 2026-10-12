# Frozen eligibility inventory v1 results

This is source eligibility and correctness evidence, not a quality study. The
selection was frozen before detector predictions: first 24 source definitions
by relative file/span in each pinned package, or all when fewer existed. It
contains 24 lz4_flex 0.14.0, 24 regex-syntax 0.8.11 and 12 time-core 0.1.9
functions, including methods, unsupported signatures and conditional absences.
The unmodified complete default-feature library targets were compiled; the 60
selected definitions are excerpts, not estimates of whole-project Rust coverage.
Archive identities, excerpt identities and licenses are in selection.json and
provenance.json. See PROTOCOL.md and AMENDMENTS.md for the frozen rules and the
disclosed host correction to aarch64-unknown-linux-gnu.

| Cumulative extraction slice | Admitted | Excluded | Unknown | Failed | Newly admitted |
| --- | ---: | ---: | ---: | ---: | ---: |
| pinned R0 baseline | 0 | 52 | 7 | 1 | 0 |
| ordinary const bodies | 0 | 51 | 8 | 1 | 0 |
| integer casts | 0 | 48 | 11 | 1 | 0 |
| scalar match | 0 | 48 | 11 | 1 | 0 |
| scalar guards | 0 | 48 | 11 | 1 | 0 |
| conditional correspondence | 4 | 48 | 7 | 1 | 4 |
| local scalar loops | 4 | 48 | 7 | 1 | 0 |
| four primitive summary instantiations | 5 | 47 | 7 | 1 | 1 |

The newly admitted functions are lz4_flex's token_from_literal,
token_from_literal_and_match_length and get_maximum_output_size in
src/block/compress.rs, time-core's cold_path in src/hint.rs, and one of the two
selected is_leap_year definitions in time-core's src/util.rs. The other
is_leap_year definition retains macro blockers. Const/cast support was necessary
for several admissions, but did not alone establish complete source evidence.
Guarded match and loops have independently verified compiler/Cargo fixtures but
unlock no selected definition in this inventory. In particular, supporting
unguarded match alone does not recover guarded lexical study examples, and
macros remain an independent blocker.

## Blockers, source-only possibilities and measured gains

Each stage JSON retains every selected function, exact source identity,
outcome, all assessment reasons, original-source syntax blockers and frontend
diagnostics. Later records also retain actual build, document-manifest and
navigation identities. summary.json compares the identical selection across
slices, lists newly admitted functions and preserves all remaining reason and
syntax combinations. Diagnostic counts are never added to claim coverage gain.

No baseline selected function was blocked solely by const, casts, match, guards,
loops or summaries: conditional/pre-expansion evidence was independently missing
for their candidate bodies. cold_path was blocked by const plus correspondence;
token_from_literal and token_from_literal_and_match_length by casts plus
correspondence; get_maximum_output_size by const, casts and correspondence.
These observed bundles unlock four functions together. After the conditional
slice, one is_leap_year definition has only primitive-call-summary blockers and
is measured as admitted after the named-summary slice. No other capability-only
unlock is observed. The preserved source-only candidate bundles are hypothetical
upper-bound inventories: unsupported interfaces, other pattern forms, callees,
conditional absence and resolution gaps can independently defeat them. They are
not predicted or measured admissions.

The seven unknown selected definitions are absent from resolved HIR under the
frozen target/features. A count_same_bytes source definition maps ambiguously to
multiple captured definitions and remains explicitly failed. Neither outcome is
fabricated into a supported function. Remaining exclusions include non-scalar
interfaces/receivers, generics, references/memory, arbitrary calls, macros and
unsupported match patterns, often in combinations. Complete reason combinations
are retained in the machine summary rather than reduced to one leading reason.

## Call selection and evidence boundaries

Source review, before accepting any predictions, froze only i32/i64 unsigned_abs
and u32/u64 wrapping_mul. The local versioned registry pins core callee paths,
definition/crate hashes, compiler commit, aarch64 target and shipped core-source
SHA-256 identities. Memory effects, mutation, panic and returning behavior are
explicit. Same-spelled user methods and neighboring primitive methods are rejected.
There is no permission for arbitrary primitive calls or source inlining.

Removed source without independently resolved facts remains unknown, including
otherwise simple cfg-disabled statements. Unsupported removed constructs are
excluded. Whole-function presence inventory covers compiler-loaded owned files;
it cannot resolve unloaded conditional modules or macro token definitions. The
loop policy excludes expression-value merges in loop-containing functions and
iterator/labeled/value exits. Cyclic dominance/reaching definitions use finite
fixed points, not sampled execution. Only actual source outcome/back/break/return
transfers exist; no executable virtual exit represents divergence. See the
versioned contract for the precise normal/exceptional-flow limits.

The real compiler tests cover exact inventories/dependencies, serialized
verification, strict imported admission and tampering. Real Cargo tests cover
opt-in/default isolation, features/edits, multi-package/target/concurrent capture,
exhaustive distinct pairs, failures and trace decision invariance. Inventory
commands use --max-pairs 0, so no detector predictions are evaluated or inspected.
A pair-budget exit 2 with complete capture/assessment is distinct from a failed
compilation or handoff. This inventory does not replace exhaustive pair evaluation
in ordinary successful clones runs.

## Historical studies and next evidence

ccgraph-v1 and ccgraph-v2, including their reports, oracles and retained
artifacts, are unchanged. The historical lz4/time and lexical examples used by
regression tests are disclosed exposed inputs. They are not new unseen holdouts
and cannot establish post-expansion pair quality.

Recommend a new preregistered quality study with untouched project/family
holdouts after these coverage corrections. Freeze source selection, labels,
matching settings and completeness rules before running it. No quality study or calibration was run, and no HIR abstraction-mining, grouping,
proposal or rewriting workflow was activated. The legacy patterns command ran
only as the explicitly requested zero-candidate style check.

## Validation commands and outcomes

The final implementation passed these actual commands on nightly-2026-09-30,
compiler commit 5c543b0b8c73c7b72bc8284ced4fb22ead15734d,
aarch64-unknown-linux-gnu (64-bit pointer layout):

- `cargo test -p rstyle-driver --test hir_extraction --locked`: 41 tests at that
  targeted checkpoint; the final workspace run includes 43 after splitting the
  independent loop-exclusion regression and adding the serialized-schema regression.
- `cargo test -p cargo-rstyle --test cli clones_named_primitive --locked` and
  `cargo test -p cargo-rstyle --test cli clones_conditional_bom --locked`: passed
  actual temporary-workspace compiler capture and admission.
- `cargo test --workspace --locked`: 493 passed across 23 suites, no failures or
  ignored tests, including 64 real-command CLI tests, 43 HIR extraction tests and
  the existing producer/imported-matching suites.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo rstyle --format json`: 1,325 functions, zero violations.
- `cargo rstyle patterns --format json --fail-on-candidates`: zero candidates.
- `uv run --no-project --with tree-sitter==0.25.2 --with tree-sitter-rust==0.24.0
  python -m unittest discover -s scripts -p 'test_*.py'`: 43 passed, including
  both study/harness suites. The initial system-Python attempt lacked sampler
  dependencies and failed; the explicitly provisioned run passed.
- The same uv dependencies with `python -m unittest discover -s eval/hir-coverage
  -p 'test_*.py'`: five independent inventory/bookkeeping/retention tests passed.

Actual per-package inventory invocations, binary identities, report/stderr hashes,
completion flags and failures are retained in the stage `*-commands.json` files.
The final stage labels are conditional-validated, loops-validated and
calls-validated. Their profiles are hir-conditional-v1, hir-loops-v1 and
hir-calls-v1, respectively. Assessment uses the same frozen selection with
`assess.py`; summary generation validates source-identity equality across stages.
Raw captured reports/stderr and selected admitted-function assessment, manifest,
PDG and CFG bytes are compressed without transformation in retained/;
retained-index.json records hashes of both compressed and original bytes. These
are retained capture artifacts, never an extraction cache or discovery source.

Earlier unsuccessful style/clippy checks and the workspace run disrupted by a
concurrent executable rebuild were development failures; their causes were fixed
or isolated before final validation. No real-compiler tests were silently skipped.
The unchanged pinned vendor and both historical study trees were checked against
main. No main/master push or merge was performed.

Remaining unverified obligations are cross-target compiler coverage beyond the
actual 64-bit host, separately reviewed summary provenance for other targets or
compiler/core versions, independent resolution for removed cfg source, and a
separately versioned cyclic expression-value merge policy. Those are not claimed
as implemented coverage or external compat approval. General exceptional-flow
and rewrite-safety completeness remain outside this task.

The independent retention checker was also run with
`uv run --no-project --with jsonschema==4.25.1 python
 eval/hir-coverage/verify_retained.py`: all 254 retained artifact identities and
manifests checked, with 76 exact serialized documents satisfying the unchanged
pinned PDG/CFG schemas. A real-compiler regression additionally checks actual
serialized documents for every selected local capability, including a cyclic
loop and an expression-valued guarded match, against those pinned schemas.

The final expanded multi-package/all-target test evaluates all 28 pairs of eight
admitted functions, compares the entire matching report under --jobs 1 and 2,
and confirms release package selection produces six pairs. The exercised sources
combine const qualification, casts, active cfg, guards, cyclic scalar state and
a verified primitive summary; an unsupported neighboring method remains excluded.
Final workspace/style/clippy output bytes and their hashes are in validation/
and validation.json. Historical R0 conformance wording is preserved exactly.
