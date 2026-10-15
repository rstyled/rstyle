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

## Implementation scope and status

1. Retire library-file reads/fingerprint gating with an explicit versioned
   supported-toolchain contract. First implementation increment; measurement and
   validation are recorded in `library-resolution-retirement.md`.
2. Pair-local complete-key location interning is implemented in the next increment;
   see `local-location-handles.md`. Unit-owned exact build-setting interning with
   scoped `BuildId` is implemented in the following increment; see
   `unit-build-contexts.md`. Source/package ownership remains pending.
3. Shared census ownership and policy/analysis-local checked source environments
   are implemented in the next increment; see `shared-source-environments.md`.
   Declaration origins now use borrowed typed frontend records; see
   `typed-declaration-origins.md`. Frozen origins now share exact census file
   buffers; see `shared-origin-buffers.md`.
   Immutable function ranges and memoized actual range identities are implemented
   next; see `owned-source-ranges.md`. Source wire dictionaries, capture-time
   reconstruction, remaining declaration identities and boundary reuse are still
   pending. Checked declaration-file inventories now consolidate nominal-binding
   hashes/parses and avoid temporary budget buffers; see
   `origin-file-inventories.md`. Continue consolidation without crossing edits, owners or strict/generated policy.
4. Replace local unit/function/ledger JSON-to-digest identities with explicit
   namespace-qualified typed handles, run-local assignment and exact duplicate
   descriptor checks. Version exports and state stability guarantees.
   Report-local function handles are implemented in `report-function-handles.md`;
   unit-store identities and capture ledger routing remain pending.
5. Replace nonsecurity transport checksums consistently, with a measured fast
   streaming checksum and honestly named/versioned fields. Keep one publication
   and one exact-snapshot import check; retain completeness, flush and association.
6. Benchmark full-key WL interning using FNV-1a and an appropriate alternative,
   including deliberate collisions. Keep a measured winner over the tree; retain
   partitions, direction, families, multiplicity, scores and deterministic reports.
7. Measure and reduce matching scratch allocation, clones, repeated graph
   preparation and trace refinement. Pair-dependent labels remain pair-local.
8. Diagnostic content fingerprints are replaced by process-local observation
   IDs in the next increment; see `diagnostic-observation-ids.md`.
   Historical study snapshots, fixture hashes and precomputed contracts are not
   ordinary-runtime costs and remain outside this optimization.
9. Minimal/read/inherent and broad fanout/many-pair/larger-graph before/after
   measurements, trace modes and strict replay are recorded for the source
   increment. Continue measurements for remaining identity/transport/matcher
   increments, including interner alternatives, collisions and allocation proxies.
10. Validate every semantic increment with focused tests and both quality gates.
    Finish with formatting, warning-free Clippy, full nextest/doctest wrapper and
    affected Python tests. Zero violations and actionable candidates are required;
    no new ignores, suppressions, changed thresholds or excluded slow families.

No entry is completed merely by documenting it. Remaining entries require code,
regression evidence and measurements before overall completion is claimed.
