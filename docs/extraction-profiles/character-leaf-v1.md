# Character leaf v1 — acceptance contract

Status: connected development implementation validated; original acceptance pending. Roadmap P01/V10; V11 casts remain
outside this increment.

## Original and context

Unchanged regex-syntax 0.8.11 `src/lib.rs::is_meta_character(char) -> bool`, lines
262–268, file byte span `[10733,10950)`:

- Function SHA256: `0f2fcf316d20d0343f8433184d833380f8b45b9761ce7b268555e58fd89184ef`.
- File SHA256: `c51d1e55a8b6c4608e21a278ed0ef9480f73ab5b814b6ca6127f4a049c4d5007`.
- Cargo manifest SHA256: `10c3f14629c0900ac452dd6d55a62aab561bbabbdf5774a802c8d223933cfa3b`.

Baseline retained report: `target/hir-coverage/run-aggudbe2/hir-data-v3/regex-syntax-0.8.11/target/rstyle/clones/run-NWAHAm/report.json`,
`function:u0:f951`; sibling `invocation-wnwh5x/unit.evidence.json`, function951.
Definition identity: `DefPathHash(Fingerprint(1678334551122178109,15612668155633592516))`.
This is complete lib/non-test capture, edition2021, default/std/all Unicode
features, aarch64-unknown-linux-gnu, rustc commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`, debug, overflow checks enabled,
panic=unwind. New evidence must retain the unchanged original and exact build
context; cross-target/std/no_std checks are additional contexts.

The source-only [selection](../../eval/coverage-development/character-leaf-selection.json)
freezes the positive and three neighboring negative controls before fresh original
admission measurement. The frozen sixty remain unchanged.

## Bounded support and blockers

Support concrete Unicode scalar interfaces/local values, original char literals,
builtin equality/order comparisons, and literal/OR/wildcard matching. Char stays
a distinct domain; numeric casts, ranges, arithmetic, Unicode APIs and generic
constants remain unsupported. This target performs no calls, memory reads,
mutation, panic or divergence.

Known chain: driver char normalization and concrete observation → source literal
and OR capture → independent spelling/type/value correspondence → char interface,
comparison and match eligibility → producer/document policy checks → strict
admission and selected matching. Existing final-wildcard exhaustiveness needs no
Unicode interval redesign. No module/import/helper blocker was observed.
Development capture retains exact literal spelling, independently checks builtin
comparison dispatch, and keeps `scalar()` unchanged. Standalone char vocabulary
requires its own capability; structured char types remain rejected at observation
and token validation. Full-original gains/losses remain to be measured. Preserve historical policy rejection; do not expand those policies.

## Independent expectations

The numeric-set oracle in
[`character-leaf-oracle.rs`](../../eval/coverage-development/character-leaf-oracle.rs)
checks every valid Unicode scalar. True exactly for eighteen code points:
`23 24 26 28 29 2A 2B 2D 2E 3F 5B 5C 5D 5E 7B 7C 7D 7E` (hex).
All other valid scalars return false, including NUL, newline, apostrophe, `%`,
`/`, `!`, `"`, `e`, U+D7FF, U+E000 and U+10FFFF. Surrogates are not char values.
The oracle links the original library; it does not reproduce its match arms.

Development expectations additionally cover equivalent escaped literals
(`'\\'`, `'\n'`, `'\x41'`, `'\u{1F600}'`) and ordered comparisons across the
surrogate gap. Malformed/out-of-domain literals must never certify. Tampering
with spelling/value/type, OR children, wildcard, source span or builtin dispatch
must fail independent verification after ordinary integrity resealing. Historical
policy downgrade and unsupported casts/ranges/APIs remain negative controls.

Delivery requires fresh complete-original capture, independent strict replay,
complete selected matching, unchanged-selection gain/loss measurement, integrated
quality gates and bounded performance controls. Development checks admit six functions and reject seven neighboring cases; all
fifteen selected comparisons complete and strict import passes. The exhaustive
original numeric-set oracle checks all 1112064 valid Unicode scalars. Workspace validation passes: 939 tests (five existing skips), doctests, formatting,
Clippy, zero rstyle violations across 3051 functions and zero unignored patterns.
Complete-original capture, strict replay and performance remain pending; no
delivered original support is claimed yet.
