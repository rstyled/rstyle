# Hash and matching requirement audit

Branch `perf/hash-and-matching-optimization` starts at user-authorized
`9cb2b32d50f30eed075e9903139058937344c20d`, since fetched main lacked the shared
store prerequisites. Executable hashing was already removed by `24f42fa` and
single-snapshot import established by `9cb2b32`; neither was reimplemented.
There is no fingerprint option. Controlled studies independently snapshot exact
binaries and retain drift checks. The untracked user file
`crates/rstyle-pdg/src/slice_access/documents.rs` remains outside task commits.

| Requirement | Current implementation and evidence |
| --- | --- |
| 1–3: baseline, workflow, trust | [Plan](hash-and-matching-plan.md) records refs/base and semantic increments. Supported local rustc/sysroot is trusted; arbitrary imports remain independently checked. |
| 4: library artifact scans | [Retirement](library-resolution-retirement.md): no normal `.rmeta`/`.rlib` fingerprint reads; versioned compiler contract, resolved crate/definition, signature and receiver checks replace binary-digest gating. Custom modified sysroots lose exact-artifact detection and are outside support. |
| 5: internal identity | [Locations](local-location-handles.md), [build contexts](unit-build-contexts.md), [function handles](report-function-handles.md), [unit namespaces](unit-namespaces.md), [ledger IDs](capture-ledger-ids.md): complete typed keys and scoped IDs replace JSON-to-SHA bookkeeping. Namespaces and exact descriptor checks preserve association; dense IDs are not persistent content hashes. |
| 6: source ownership | [Snapshots](source-snapshot-identity.md), [ranges](owned-source-ranges.md), [declarations](declaration-ranges.md), [manifests](manifest-owner.md), [capture manifests](capture-manifests.md), [checked environments](shared-source-environments.md) share immutable bytes and established identities. Independent owners, source edits and strict/generated policy retain separate checks. |
| 7: hash-only serialization | Local location/build/function/unit/ledger identity serialization is gone. Real report/bundle serialization remains at boundaries. Source digest memos hash exact original bytes; no intermediate JSON encoding is needed. |
| 8: transport | [XXH3-64](transport-checksum-v2.md) replaces normal nonsecurity checksums with explicit versioned algorithm fields. Seal computed during checked publication is verified once against the buffer subsequently decoded. Inventories, final flush, run/unit association and incomplete-publication rejection remain. Cryptographic collision resistance is deliberately retired; checksums do not authenticate. |
| 9: exact interning | [Selection](wl-interner-selection.md) compares complete-key FNV and XXH3 maps with tree variants. Entry-tree is the measured choice; forced-collision regression proves complete equality. No SHA-to-FNV matching conversion occurred: core already used FNV and strict matching already used exact tuples. |
| 10: preparation/allocation | [Integer scoring](integer-scoring-labels.md), [label buffers](wl-label-buffers.md), [shared trace](shared-wl-trace.md) remove string histogram copying, repeated label allocation and duplicate refinement. `pipeline::prepare_all` prepares graph projection/adjacency/order/diameter before pair enumeration. Pair-dependent labels remain pair-local. [Diameter scratch](diameter-scratch.md) was rejected after mixed results. Owned neighbor vectors remain a measured structural allocation cost. |
| 11: diagnostics | [Observation IDs](diagnostic-observation-ids.md) replace diagnostic evidence hashes. Persisted source-inventory digests and frozen registry/fixture identities have separate consumers. |
| 12: correctness | Full wrapper retains exact-score/boundary, multiplicity, self-edge, reordered/renamed inputs, source-edit, malformed-import, context and policy-isolation regressions. No thresholds, pair enumeration, supported profiles or budgets were changed for speed. |
| 13: measurements | Linked increments retain raw ordinary/diagnostic samples for minimal/read/inherent/fanout/larger graphs/many pairs, debug/release, trace modes and strict replay. Each increment states its actual baseline; results must not be added together as independent gains. RSS is a whole-process proxy rather than an allocator census. |
| 14–15: validation/delivery | Final command results and remote revision accompany the completion report. Historical per-increment results validate their stated revisions only. |

## Retained fingerprints and boundary work

Exact original source, function-range, declaration and manifest identities remain
where persisted source correspondence requires them. Their immutable owners memoize
actual identities, and public independently callable validators still check supplied
claims. Conditional source inventories persist distinct definition/function source
identities; these are not timing labels. Compiler crate/definition identities remain
compiler evidence, not a home-grown binary fingerprint cache.

The standalone serialized-pair API retains its SHA-256 PDG/CFG manifest contract.
Normal shared bundles do not invoke that serializer. Frozen contract constants,
fixture packs and independent Python experimental snapshots remain outside ordinary
analysis. No replacement version string or fabricated value occupies a SHA field.

Internal bundle v4 transports source dictionaries/ranges; closed matching-build v1
records only the three settings required by matching compatibility, while full build
and source evidence stays in the owning frontend. These are intentional format/API
changes, not a permanent parallel legacy runtime. All changed format contracts and
exported ID stability limits are documented in the linked increments.

## Limits

Remaining costs include rustc/Cargo startup and capture, actual boundary encoding
and semantic verification, report output, owned refinement tuple neighbors, directed
diameter traversal and exhaustive pair work. Hash maps and scratch reuse did not
consistently beat the retained implementation. No universal runtime improvement or
whole-suite speedup follows from a local sample, and no memory figure below is an
exact allocation count.

## Semantic commit ledger

The following implementation commits are descendants of the authorized base.
Subjects are limited to 70 characters; final audit/benchmark commits follow them.

- `3f6e763` Replace library artifact fingerprints with compiler resolution
- `5bf2e4e` Intern formal locations with pair-local typed handles
- `2a25005` Intern exact build contexts in unit-owned report tables
- `3211768` Use observation IDs instead of diagnostic evidence hashes
- `9c05c40` Share checked source environments within immutable analyses
- `5444e70` Borrow typed declaration origins during analysis
- `53f7123` Share immutable source buffers with declaration origins
- `b0d239c` Own function source ranges and reuse actual byte identities
- `c0b9cb7` Reuse checked declaration files within each binding context
- `5a27ebe` Use qualified report-local function handles
- `33b7a4d` Replace frontend unit fingerprints with local namespaces
- `d6b9522` Route capture ledgers through frozen-plan target IDs
- `e9e82a3` Use explicit fast checksums for local evidence transport
- `f8f494d` Select entry-based WL interning from measured alternatives
- `2a19b63` Score WL histograms with borrowed integer labels
- `9a9464c` perf: collect WL trace partitions during scoring
- `f2fbf16` perf: validate selected context once per imported unit
- `6762c93` perf: reuse pair-local WL label buffers across rounds
- `caa1cd9` perf: share original compiler sources during unit capture
- `a4666ab` perf: transport function source snapshots through local ranges
- `4b106f8` perf: share census and origin sources in frontend bundles
- `330b7d2` perf: memoize actual file identities in shared source owners
- `3444d4d` bench: record diameter scratch reuse tradeoffs
- `3ed20c5` perf: reuse immutable package manifest identities
- `20a39c6` perf: share checked function source ranges with CFG content
- `869d88f` perf: keep graph build metadata to compatibility settings
- `d1c7fa8` perf: share declaration ranges and memoized source identities
- `f03e721` perf: reuse fallback manifest snapshots within compiler capture

## Final validation

After the capture-manifest implementation and isolated-backend benchmark seam:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `CARGO_TARGET_DIR=target timeout 1800 ./scripts/test.sh --nextest`: 698 passed,
  zero skipped, 32.090 s observational; workspace doctests passed.
- `./scripts/style.sh --format json`: 2,106 functions, zero violations.
- `./scripts/style.sh patterns --fail-on-candidates --show-ignored`: passed,
  zero actionable candidates, same 15 reviewed ignores.
- Study Python tests: 24 passed; measurement-harness Python tests: two passed.
- Standard before/after parity: 120 matching reports and 2,160 frontend
  records/options identical after run-path normalization; strict replays passed.
- Targeted helper capture: same matching, outcomes, sources, signatures and
  completeness; medians unchanged (debug 0.37 s, release 0.24 s).

Logs are `/tmp/rstyle-final-hash-*`. Builds, tests, style checks and ordinary
measurements were sequential. No ignores, suppressions, thresholds or slow-family
exclusions were added. Measurements and caveats are in the linked reports; the
executable-only predecessor measurement is in
[executable hash retirement](executable-hash-retirement.md).
