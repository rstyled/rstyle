# Shared evidence ownership work

Branch: `perf/shared-evidence-store`. Base: fetched `main`
`045d4ac331babca8dbc8d33867d7044861ee70a2`. The clone-stabilization
prerequisites are already merged into this base; no feature history is imported.
The optional in-process-analysis proposal is absent from this checkout.

## Local baseline

Linux aarch64, cgroup CPU quota 8 cores, memory limit 16 GiB, pinned
nightly-2026-09-30 (`5c543b0b8`), locked dependencies, unoptimized example
profile. Build time is excluded. One process, no concurrent test workload,
warm build artifacts. Input is the retained 830,724-byte, 21-function
pre-helper-certification snapshot described in clone-stabilization.md:
`/tmp/rstyle-perf-inherent-raw/1595339-pre-helper-certification.json`.
This sandbox-local artifact is not a portable regression fixture.

Command: `/usr/bin/time -v timeout 120 target/debug/examples/helper_replay
<snapshot> --generated`, with detailed instrumentation disabled. One observation:
26.17 seconds wall time, 26.09 seconds user CPU, 118,708 KiB peak RSS. The
helper stage completed for all 21 functions; this is not full production or
matching acceptance. Logs: `/tmp/rstyle-shared-before-replay.{json,log}`.
Historical measurements are not used as a comparative baseline.

Baseline wrappers both complete successfully: 1,782 functions and zero style
violations; zero actionable pattern candidates, with 17 existing reviewed
ignores. Reports: `/tmp/rstyle-shared-baseline-style.json` and
`/tmp/rstyle-shared-baseline-patterns.log`. The initial wrapper invocation
failed because Cargo was absent from PATH; the successful runs explicitly add
`/home/dev/.cargo/bin` to PATH.

Code inspection confirms that capture clones prepared frontend trees and
completed dependencies, verification copies serialized PDG/CFG buffers, and
admission clones both complete documents. The per-call JSON transport still
expands dependencies on decode. Counts of those structural operations are
code observations, not measured allocation totals.

## Capture construction increment

Capture prepares each pending function in its existing mutable unit buffer,
then moves completed dependencies into the certification map. It no longer
clones each pending frontend tree or each completed full dependency. Round
boundaries still prevent same-round discoveries from changing certification
order. Missing/duplicate completed definition identities are checked before
publication. As before, a failed capture can leave partially attached evidence;
callers must not publish a failed capture.

This increment does not normalize the nested transport: certification still
copies facts into the dependency required by that format. No ID or wire-format
guarantees change yet. Focused PDG tests and all nine `clones_helpers` CLI tests
pass, including transitive edits, tampering, recursion exclusions, capture order
and argument positions. Both wrappers pass: zero style violations and zero
actionable candidates, with the same 17 reviewed ignores.

## Borrowed serialized verification increment

`SerializedPairView` borrows PDG bytes, CFG bytes and the manifest from one
owned snapshot. `verify_serialized_view` runs the same byte identity and
strict/generated checks as the owning adapter. Helper callee verification now
uses this view instead of allocating two byte buffers and cloning a manifest.
Manifest comparison sorts borrowed entries and retains duplicate/inventory
checks. Ordinary deserialization still creates typed documents; this API does
not claim zero-copy decoding or a shared-table import.

All PDG tests and ten focused helper/policy CLI tests pass. The borrowed-path
regression checks output byte parity and rejects a whitespace-only byte edit
with the unchanged manifest under both policies. Workspace all-target Clippy
passes with warnings denied. Both wrappers pass with zero violations/actionable
candidates and 17 existing ignores. No schema or identity meaning changes.

## Compact prepared matching increment

Prepared matching borrows the admitted `ImportedFunction` rather than cloning
its complete inventory. After feature extraction, it replaces the projected
string graph with immutable contiguous labels and edges with distinct `u32`
node/edge handles. Incoming and outgoing CSR tables contain occurrence IDs;
parallel occurrences and self-edges remain separate. Dense label vectors and
adjacency slices serve both diameter and WL. Each production projection is
converted once and reused for every pair. Public readable stage-fixture APIs
adapt to the same computational implementation; optional partition exports
resolve names only when requested.

Handles are private computational details, numbered in sorted readable-name
order. They are valid only within their owning immutable graph, not persistent
definition/evidence identities or cross-unit references. All index/count
conversions to u32 are checked. Duplicate nodes and dangling endpoints fail
construction. There is no new graph wire format; imported names, provenance,
strong digests and existing readable exports retain their meaning. Public
infallible WL/diameter fixture adapters require valid projected graphs, as the
production pipeline already ensures.

Five alternating executions of the unchanged 120-node, 952-occurrence parallel
chain regression, using stable before/after test executables on this machine,
measure supervisor medians of 76.33 ms before and 5.54 ms after. Build time is
excluded; process/test harness startup is included, instrumentation disabled,
no other test workload runs concurrently. Samples:
`/tmp/rstyle-shared-graph-samples.json`. A coarse `/usr/bin/time -v` observation
reports 3,688 KiB peak RSS for both. This small test is not a whole-analysis
speedup claim. Structural work changes from scanning every edge at each BFS
vertex to scanning that vertex's outgoing slice; WL no longer sorts/clones
node names or rebuilds string-keyed neighborhoods each round.

All CCGraph tests (including exact fixture partitions, renaming, order,
multiplicity and score boundaries), three new compact-storage regressions,
and ten helper/policy CLI tests pass. Workspace Clippy passes. Both wrappers
pass with zero style violations and zero actionable candidates. The unchanged
ignore ledger now hides 16 findings: reviewed candidate `0ee0a828b288` disappears
because the old string BFS successor scan was removed. No ignore is added,
deleted or modified.

## Remaining ownership boundary

The primary shared-unit objective is not yet implemented. Certification still
copies frontend facts into the legacy dependency record, callers still carry
per-call JSON, imported verification still repeatedly decodes that transport,
and admission still clones full document pairs. There is no shared-table unit
bundle, typed helper-link contract, or resolver-backed unit import yet. The
changes above must not be described as satisfying unique-evidence ownership
or the complete requested refactor.
