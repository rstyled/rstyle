# Frozen eligibility inventory v1 results

This is eligibility and correctness evidence, not a quality study. At implementation
commit `1a6b2d205c4774d0af29cb069ee0d2ace7b23764`, the frozen 60-definition inventory
produced the following results. Selection is the first 24 definitions sorted by
relative file/span in lz4_flex 0.14.0 and regex-syntax 0.8.11, and all 12 in
time-core 0.1.9. Unmodified complete default-feature library targets were compiled;
these selected excerpts do not estimate whole-project Rust coverage.

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

The newly admitted functions are lz4_flex's `token_from_literal`,
`token_from_literal_and_match_length` and `get_maximum_output_size`, time-core's
`cold_path`, and one of its two selected `is_leap_year` definitions. The other
variant retains macro blockers. Const/cast support was necessary for several
admissions but did not alone establish complete source evidence. Match, guards
and loops unlock no additional selected definition. Their correctness is checked
by independent real-compiler fixtures, not inferred from this inventory.

No baseline selected function was blocked solely by one proposed capability:
conditional evidence was also missing. After the conditional slice, the admitted
`is_leap_year` variant had only call-summary blockers. Diagnostic counts are never
summed to claim coverage gains. Seven selected definitions are absent from HIR
under the frozen target/features and remain unknown. `count_same_bytes` maps
ambiguously and remains failed. Other exclusions include unsupported interfaces,
generics, references/memory, arbitrary calls, macros and patterns, often together.

## Reproduction and maintenance

Run `uv run eval/hir-coverage/reproduce.py` from a checkout with rustup and the
pinned aarch64 toolchain available. See [README.md](README.md) for requirements and
output layout. The runner verifies package archives, dependency lockfiles and the
complete frozen selection, builds the tools, captures every profile, checks actual
serialized documents against exact manifests and unchanged pinned schemas, and
regenerates per-function assessments and the coverage comparison. All generated
outputs belong under ignored `target/hir-coverage/`, not in Git. Compare semantic
outcomes and source identities; paths, binary hashes and log bytes may change.

The previously committed raw capture archives, repeated stage reports and
superseded development attempts have been removed. Inputs, contracts, independent
expectations and reproduction code remain. Runtime manifest verification, frontend
verification and strict producer/imported admission are unchanged. The runner
uses `--max-pairs 0`; it checks that no predictions were evaluated and distinguishes
the deliberate pair budget from capture failures. Successful ordinary clones runs
still require exhaustive pair evaluation.

## Validation and limits

Cleanup validation reran `uv run eval/hir-coverage/reproduce.py`: all 24 actual
Cargo captures completed, all eight profile counts reproduced the table, and 28
serialized documents passed exact-manifest and pinned-schema verification. The
five inventory and 43 study/harness scripting tests passed. Both targeted Rust
regressions passed:

- `cargo test -p rstyle-driver --test hir_extraction expanded_construct_documents_satisfy_unchanged_pinned_schemas_as_serialized --locked`
- `cargo test -p cargo-rstyle --test cli clones_expanded_multi_package_targets_are_exhaustive_and_schedule_invariant --locked`

`cargo fmt --all -- --check`, `cargo rstyle --format json` and
`cargo rstyle patterns --format json --fail-on-candidates` passed with zero style
violations and zero candidates. No Rust implementation changed in this cleanup;
the full workspace and Clippy results below belong to the implementation commit.

Implementation validation at the commit above passed 493 workspace tests,
including 64 real-command Cargo CLI tests and 43 pinned-compiler extraction tests;
formatting, strict Clippy, 43 study/harness scripting tests and five inventory tests
also passed. `cargo rstyle` reported zero violations across 1,325 functions, and
`cargo rstyle patterns` reported zero candidates. Multi-package scheduling checks
evaluated all 28 pairs of eight admitted functions with identical reports.
These are reported validation results, not permanent output fixtures.

Removed source without independent resolved facts remains unknown. Loop-containing
functions with expression-value merges are excluded. Named summaries are limited
to i32/i64 `unsigned_abs` and u32/u64 `wrapping_mul`, with pinned core/compiler
provenance on aarch64-unknown-linux-gnu. Other targets/compiler versions, removed
cfg resolution and cyclic expression-value merges remain unverified. General
exceptional-flow and rewrite-safety completeness are outside scope.

ccgraph-v1 and ccgraph-v2 remain unchanged historical studies. Exposed historical
inputs used in regression tests are not unseen holdouts. No calibration or HIR
mining/grouping/proposal/rewriting workflow ran. A new preregistered quality study
using untouched project/family holdouts is recommended separately.
