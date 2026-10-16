# Shared-evidence follow-ups

For the subsequently implemented hash/matching task, see the
[requirement audit](hash-and-matching-acceptance.md). The optional priorities below
are historical shared-store follow-ups; source dictionaries and scoped ownership
are now implemented, and executable hashing has since been removed entirely.

The original shared-evidence and compact-graph implementation has no outstanding
acceptance items. The historical [acceptance audit](shared-evidence-audit.md)
records its original revision; subsequent work retired expanded evidence,
streamed executable hashing, and removed repeated fixture imports. All required
quality gates remain mandatory for further changes.

## Completed follow-ups

- `2c27062`: retire expanded-evidence production, export and consumers.
- `cb6464c`: stream full executable hashes, preserving content invalidation.
- `ba4ec97`: borrow family-test evidence across read-only helper assertions.
- `fcaed29`: reuse unit analysis across remaining per-function fixture loops.
- Borrow CLI invocation and selected-unit JSON during validation: deserialize
  directly from existing values rather than cloning the trees first. Each
  function still undergoes its own checks. Typed invocation strings and unit
  contexts are still decoded and owned; this does not intern them or cache
  validation. Data functions avoid two temporary JSON-tree clones; other
  functions avoid one. Formats, identities and admission checks are unchanged.
  This structural reduction carries no timing or allocation-count claim.

## Optional work, in priority order

1. The [initial production profile](production-profile.md) is complete for the
   retained helper workloads in debug and release. Next split compiler capture
   into orchestration, HIR extraction, certification and publication, and
   attribute strict source/type verification. Keep ordinary timing separate
   from diagnostics before another architectural change.
2. If profiling justifies it, share repeated build/source context within a unit.
   Preserve package/type context, exact source bytes, occurrence identity,
   independently checked evidence and strict/generated policy isolation.
   Source/context dictionaries are not implemented or required for acceptance.
3. Measure serialization and report costs before considering binary encoding,
   compression, columnar storage, mmap or streaming stdout. These are separate
   format/API projects, not prerequisites for the shared representation.
4. Optimize remaining legitimate graph/pair work only when measurements support
   it. Exhaustive pair enumeration, exact decisions, completeness and existing
   budgets must remain. Quadratic pair enumeration alone is not a regression.

These are optional investigations, not promises that every item should be
implemented. Test runtime improvements above do not imply production speedups.
The user's untracked `slice_access/documents.rs` is outside this work.

## Borrowed CLI handoff validation

Validated against baseline `fcaed29` with the pinned toolchain and locked
dependencies. No rules, ignores or test settings changed. Commands ran
sequentially and all exited successfully:

- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `CARGO_TARGET_DIR=target timeout 600 ./scripts/test.sh --nextest`: 642 passed,
  zero skipped, 106.550 seconds, no slow warnings; workspace doctests passed.
  This includes existing all-target, conditional-invocation, nominal-context,
  malformed/tampered handoff and strict/generated policy regressions.
- `./scripts/style.sh --format json`: 1,970 functions, zero violations.
- `./scripts/style.sh patterns --fail-on-candidates --show-ignored`: zero
  actionable candidates, 15 existing reviewed ignored candidates.

Logs: `/tmp/rstyle-borrow-handoff-{fmt,clippy,nextest,patterns}.log`,
`/tmp/rstyle-borrow-handoff-style.json` and
`/tmp/rstyle-borrow-handoff-style.err`. Suite duration is observational; no
controlled performance samples were collected for this small clone removal.
