# Hash and matching optimization implementation

Branch: `perf/hash-and-matching-optimization`.
Exact base: `9cb2b32d50f30eed075e9903139058937344c20d`.
Fetched main was `045d4ac331babca8dbc8d33867d7044861ee70a2` and did not contain
the prerequisites. The user explicitly authorized basing this new branch on
`origin/perf/shared-evidence-store` at `9cb2b32` instead. No unrelated history
was imported. The user's untracked `slice_access/documents.rs` remains untouched.

The full task is implementation, not an audit. The earlier hash audit describes
historical consumers, not mandatory permanent SHA-256 requirements. The intended
policy trusts the supported locally installed rustc/toolchain; arbitrary imported
serialized evidence does not acquire that trust.

## Starting implementation verified

- `24f42fa` removes normal CLI/driver executable hashing.
- `9cb2b32` verifies a bundle once into an immutable owned byte snapshot.
- `rstyle-core` WL already uses FNV-1a; no SHA-256 matching replacement is claimed.
- Strict `rstyle-ccgraph` WL interns complete `WlTuple` keys in a BTreeMap with
  one shared vocabulary across the compared graphs. Changing its lookup table
  must preserve full equality and deterministic allocation order.
- Baseline style: zero violations across 1,970 functions. Baseline patterns:
  zero actionable candidates; 15 existing reviewed ignores, unchanged.

## Full remaining scope

1. Retire library-file reads/fingerprint gating with an explicit versioned
   supported-toolchain contract. First implementation increment; measurement and
   validation are recorded in `library-resolution-retirement.md`.
2. Intern complete typed root/projection keys into location handles; intern exact
   relevant build contexts with owner-scoped `BuildId`.
3. Consolidate source snapshots, checked ranges and source/context correspondence.
   Reuse checked views within immutable owners without crossing edits, owners or
   strict/generated policy boundaries.
4. Replace local unit/function/ledger JSON-to-digest identities with explicit
   namespace-qualified typed handles, run-local assignment and exact duplicate
   descriptor checks. Version exports and state stability guarantees.
5. Replace nonsecurity transport checksums consistently, with a measured fast
   streaming checksum and honestly named/versioned fields. Keep one publication
   and one exact-snapshot import check; retain completeness, flush and association.
6. Benchmark full-key WL interning using FNV-1a and an appropriate alternative,
   including deliberate collisions. Keep a measured winner over the tree; retain
   partitions, direction, families, multiplicity, scores and deterministic reports.
7. Measure and reduce matching scratch allocation, clones, repeated graph
   preparation and trace refinement. Pair-dependent labels remain pair-local.
8. Remove large diagnostic content fingerprints where observation IDs suffice.
   Historical study snapshots, fixture hashes and precomputed contracts are not
   ordinary-runtime costs and remain outside this optimization.
9. Complete representative minimal, read/inherent, fanout, larger graph/pair,
   trace on/off and strict-replay before/after measurements. Record structural
   counts separately from ordinary timing, allocation proxies and peak memory.
10. Validate every semantic increment with focused tests and both quality gates.
    Finish with formatting, warning-free Clippy, full nextest/doctest wrapper and
    affected Python tests. Zero violations and actionable candidates are required;
    no new ignores, suppressions, changed thresholds or excluded slow families.

No entry is completed merely by documenting it. Remaining entries require code,
regression evidence and measurements before overall completion is claimed.
