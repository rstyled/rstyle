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

## Ownership boundary after the first increments

At the end of the first three implementation increments, the primary
shared-unit objective was not yet implemented. Certification still
copies frontend facts into the legacy dependency record, callers still carry
per-call JSON, imported verification still repeatedly decodes that transport,
and admission still clones full document pairs. There is no shared-table unit
bundle, typed helper-link contract, or resolver-backed unit import yet. The
changes above must not be described as satisfying unique-evidence ownership
or the complete requested refactor.

## Repeated helper replay comparison

Three alternating baseline/current runs use the same retained 21-function
snapshot, generated policy, profile/toolchain, warm executable caches and
single-process concurrency, with detailed instrumentation disabled. Baseline
is rebuilt from the exact base in detached worktree
`/tmp/rstyle-shared-baseline-worktree`, using a separate target directory;
current uses the workspace example. Build time is excluded. Each replay has
a 120-second timeout and all six complete successfully. Python supervisor
timings include process startup, `/usr/bin/time` and timeout setup:

| Measurement | Base | Current |
| --- | --- | --- |
| Wall samples (seconds) | 26.154, 26.109, 26.073 | 26.358, 26.224, 26.366 |
| Median wall (seconds) | 26.109 | 26.358 |
| Peak RSS samples (KiB) | 119796, 119796, 119680 | 120796, 120788, 120884 |

This demonstrates no helper-stage speedup or memory reduction. The main nested
transport costs remain. Removing identified clones is a structural reduction,
not proof of better whole-helper performance. No allocation counts or transport
decode/encode counts were collected in these uninstrumented runs; no claim of
decode-once ownership follows. Raw logs:
`/tmp/rstyle-shared-replay-{before,after}-{0,1,2}.{stdout,log}` and
`/tmp/rstyle-shared-replay-samples.json`.

## Final validation of the partial implementation

The complete stable-binary wrapper run passes: 617 tests across 23 binaries,
617 passed, two slow, zero skipped, nextest wall time 247.995 seconds. The
unchanged inherent-family test passes in 236.523 seconds, including strict
post-return/tampering assertions, warm parity and source-edit propagation.
The wrapper's separate workspace doctest command passes (no current doctests).
External supervisor wall time is 248.93 seconds, including wrapper/build/runner
setup, under an explicit 600-second timeout. CI's existing four-thread profile
is unchanged. Log: `/tmp/rstyle-shared-final-nextest-stable.log`.

An initial full run failed three CLI setup tests because concurrent style
wrapper builds replaced the CLI executable while tests were executing it
(`cargo-rstyle (deleted)`, ENOENT). That run is not acceptance: 614 passed,
three failed, no doctest step reached. The entire suite was then rerun after
all builds finished, with no concurrent build/gate commands. Failed log:
`/tmp/rstyle-shared-final-nextest.log`. Assertions and test budgets were not
changed to address this setup mistake.

Commands actually run on this branch:

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `CARGO_TARGET_DIR=target ./scripts/test.sh --nextest` (600s bound) | Stable full rerun passes, including doctests |
| `./scripts/style.sh --format json` | Complete report, 1,801 functions, zero violations |
| `./scripts/style.sh patterns --fail-on-candidates --show-ignored` | Zero actionable candidates, 16 existing ignored findings |

Focused increments additionally ran all PDG/CCGraph crate tests and filtered
CLI nextest runs for helper and generated-policy regressions. Final logs use
the `/tmp/rstyle-shared-final-` prefix. Cargo's bin directory was added to PATH;
the pinned toolchain and locked dependencies were used throughout. No rule,
threshold, ignore entry, dependency or original test assertion was removed or
weakened. The pre-existing untracked `crates/rstyle-pdg/src/slice_access/`
directory is not part of these commits.

Implementation commits (subjects at most 70 characters):

- `d029c6c` Record shared evidence baseline and merged prerequisites
- `c420fc1` Move completed helper proofs and prepare facts in place
- `bc5977d` Borrow serialized helper documents during verification
- `28360b6` Store prepared matching graphs with dense handles and adjacency

The validation/documentation commit records these measurements and the explicit
incomplete shared-unit boundary; it does not introduce another runtime change.


## Unit-owned borrowed helper analysis

`UnitStore::capture` moves raw compiler records into a mutable builder, then
freezes the frontend, helper pairs and helper links in a unit-owned store.
`FunctionId` and `HelperId` are distinct checked `u32` handles with private
constructors. Resolver-backed views qualify all lookups by their owning store;
no public lookup accepts an unqualified handle from another store. Handles
remain valid for that immutable store's lifetime, with no stability promise
across inserted, reordered or edited source records. No persistent dense-ID
registry is introduced.

Raw frontend trees carry no nested proofs. Call-site side tables point to
functions and certification side tables point to unique helpers. Helper
certification owns one summary and one typed paired document per supported
function. Analysis borrows those records; it never serializes/decodes a full
helper proof on the shared path. The explicit `EvidenceFacts` context flows
through eligibility, lowering, production and verification. Standalone raw
facts use the existing strict transport adapter through the same analysis
implementation. The driver and CLI still use that adapter pending the next
handoff increment.

Frontend graph extensions hold a function reference qualified by a strong
SHA256 identity of the exact raw unit evidence, rather than embedding another
frontend copy. Existing logical definition identities remain distinct. The
unit identity changes after source/context edits or record reordering; it is
not a global logical definition identity and does not silently redefine the
legacy evidence digest. Standalone verification rejects these references;
verification with a resolver rejects foreign unit/function references,
contradictory inline facts and paired extension disagreement, even under the
trusted-generated policy. This is an internal reference profile so far, not
a published bundle compatibility promise.

Successful callee proof reuse belongs to a separate analysis context referencing
one immutable store. Strict and generated policies have separate cache entries;
caller identity/signature/dispatch/argument checks run per occurrence. A new
analysis starts with an empty cache, and policy restoration survives unwind.
Records and cache entries contain no serialized trust capability. No global
unit cache, self-referential structure or per-node `Arc` is used.

The diamond/repeated-call CLI regression checks five unique helper records,
borrowed summary pointer sharing for three calls, raw caller trees without
nested proof fields, strict/generated document parity, both capture orders,
foreign/contradictory references and transitive native-width inspection. Two
unit regressions check policy isolation, independent cache lifetimes and
unwind restoration. Source/build metadata and strings within each graph pair
are not yet normalized; this increment eliminates full dependency-tree
multiplication, rather than claiming every repeated string has disappeared.

The normalized versioned unit bundle, validated owning import, resolver-backed
readable exports and driver-to-CLI adoption remain required. This intermediate
increment does not establish end-to-end shared ownership or completion of the
requested refactor.

Three alternating before/after generated-policy replay samples use the retained
exact-main baseline executable and the current unit-store executable, both
unoptimized builds of the same pinned toolchain, same snapshot, warm artifacts,
no concurrent workload and disabled detailed instrumentation. Supervisor wall
medians including process startup and snapshot parsing are 26.354 seconds before
and 1.732 seconds after. Peak RSS samples are 119,544/119,768/119,768 KiB before
and 28,064/27,920/28,060 KiB after. Setup/build time is excluded; nested stage
intervals are not summed. Logs and samples: `/tmp/rstyle-unit-samples.json` and
`/tmp/rstyle-unit-{before,after}-{0,1,2}.log`. This is helper certification only,
not a full CLI speedup or pair-enumeration claim. The strict replay also
completed with the same 21 functions and 20 unique helpers.

A separate opt-in instrumented replay confirms no legacy `helper.verify` or
`helper.transport_encode` events on the unit-store path. Instrumented duration
is excluded from the comparison. No allocation-count claim is made. Both
wrappers pass with zero style violations/actionable candidates, and the same
16 reviewed ignores. Workspace all-target Clippy passes with warnings denied;
all focused PDG tests, cache regressions and the diamond CLI regression pass.

The full wrapper `CARGO_TARGET_DIR=target timeout 600 ./scripts/test.sh --nextest`
passes all 620 tests with no skips, followed by workspace doctests. The exact
nextest duration and slow-family timing are retained in
`/tmp/rstyle-unit-nextest.log`. `cargo fmt --all -- --check` also passes.

## Shared bundle and graph root increment

The versioned `rstyle.shared-unit/1` bundle streams one raw frontend table,
one graph table, one helper summary table and explicit typed references.
`GraphId(u32)` joins the existing scoped handles. Function roots and helper
certificates share their graph record instead of duplicating a pair. Import
moves each decoded typed record into its owning table, validates bounds,
contradictory references, unused graphs, complete root/certificate inventories,
helper cycle/depth rules and strict paired evidence. Imported verification
forces strict policy even inside a generated scope; no trust cache is serialized.
Format guarantees and remaining driver/CLI adoption are documented in
[shared-unit-bundle.md](shared-unit-bundle.md).

The diamond regression now round-trips the bundle byte-for-byte, reverses and
renumbers graph/helper tables while preserving nodes/edges and production
results, and rejects schema, wrong-kind, dangling function/helper/graph links,
contradictory call links, invalid depths, changed summaries, missing certificates
and paired graph disagreement. Existing standalone tampering assertions remain.
This increment does not yet change driver publication or CLI artifact loading.

Focused PDG tests and the bundle/diamond CLI regression pass after the graph
root changes. Formatting and workspace all-target Clippy pass. Both wrappers
complete with zero style violations and zero actionable candidates, retaining
16 existing ignored findings. Reports are `/tmp/rstyle-bundle-style.json`,
`/tmp/rstyle-bundle-patterns.log`, `/tmp/rstyle-bundle-clippy.log` and
`/tmp/rstyle-unit-bundle-tests.log`. The 620-test full suite reported above
preceded this bundle increment; it is not presented as validation of new bundle
code. Final end-to-end validation still requires driver/CLI integration.

## Legacy expansion adapter increment

The old capture-state/certification loop is removed. The mutable-facts API now
constructs the same shared store, then invokes a separate optional standalone
export. This compatibility boundary necessarily copies raw frontend facts and
expands proof strings; those costs are not part of shared-store analysis.
The driver/CLI still need to stop requesting this expanded contract.

Expansion follows validated helper depth order, caches each unique legacy
proof encoding within the export, retains the existing depth-eight and 8 MiB
per-call limits, and adds an explicit 512 MiB aggregate work budget. It counts
UTF-8/JSON bytes through an in-memory serializer and fails without publishing
partial output. The legacy mutable input is updated only after a complete
successful export. Restoration keeps its unchanged 8 MiB budget and error text;
a common checked subtraction primitive serves both budgets. No thresholds,
ignore entries or analysis capability checks change.

All focused PDG tests (including UTF-8 budget exhaustion) and nine existing
helper CLI regressions pass. Formatting, workspace all-target Clippy and both
wrappers pass, with zero style violations/actionable candidates and the same
16 reviewed ignores. Logs: `/tmp/rstyle-expanded-{pdg-tests,helper-tests}.log`,
`/tmp/rstyle-expanded-style.json`, `/tmp/rstyle-expanded-patterns.log` and
`/tmp/rstyle-expanded-clippy.log`. The broader final suite remains outstanding
for this adapter and the forthcoming publication integration.

## Borrowed admission context increment

`admit_produced` accepts the explicit borrowed evidence context and passes it
through admission and native-width propagation. After producer verification,
it retains profile agreement, PDG/CFG validation, correspondence and projection
checks without reconstructing another owning pair or decoding embedded frontend
facts. Standalone strict document admission retains its narrow frontend adapter.
The shared diamond and renumbering regressions compare complete admitted
functions exactly with standalone expanded evidence, not just timings.

Focused admission/diamond coverage and all CCGraph tests pass. Formatting,
workspace Clippy and both wrappers pass, with zero violations/actionable
candidates and 16 existing ignores. Logs: `/tmp/rstyle-borrowed-admission-tests.log`
and `/tmp/rstyle-admission-{style.json,patterns.log,clippy.log,ccgraph-tests.log}`.


## Normalized compiler-to-CLI handoff

The ordinary CLI explicitly requests one `rstyle.shared-unit/1` bundle and
`rstyle.shared-unit.manifest/1` byte seal per selected compiler unit. Driver
capture moves facts into the store and streams unique frontend/helper/graph
records. CLI loading checks inventory and seal against one owned byte snapshot,
then moves its decoded records into one store. Function reports share the whole
store across lifetime boundaries; analysis borrows records from one scoped
resolver. Direct standalone driver output remains a narrow export adapter.
`--expanded-evidence` explicitly enables bounded readable standalone exports;
normal capture/import/admission does not reconstruct nested dependencies.

Generated current-run loading preserves the independently frozen producer and
selected-unit inspection. Ordinary imports always run strict verification.
Serialized flags never confer trust. Exact transport sealing remains separate
from the raw semantic unit identity and existing paired manifest identities.
The evidence-view trait is sealed against foreign-owner helper substitutions.
Successful native-width results are memoized per resolver/function, without
caching errors or mixing generated/strict proof caches. The depth-eight,
six-calls-per-level regression admits all eight functions and evaluates all
28 pairs without expanding the helper closure. R0 retains its existing closed
helper capability rather than acquiring helper certification implicitly.

Storage comparison on the same 21-function snapshot, debug pinned toolchain,
uses the optional replay `--generated --storage` mode. It completes with 20
unique helper records and 21 unique paired graphs. The normalized bundle is
2,985,484 bytes. Its equivalent standalone compatibility export is 4,314,745
frontend bytes plus 17,336,189 paired-document/manifest bytes: 21,650,934 total.
Both counts use the current serializer and the same evidence; this is a
representation comparison, not a historical baseline-file or timing claim.
Separate inventory/seal files are excluded from both counts. Optional expansion
and re-production are measurement work and are not included in ordinary replay
samples. Logs: `/tmp/rstyle-storage-comparison.{jsonl,log}`.

Focused PDG/seal, snapshot, default/shared-fanout, helper and tampering coverage
passes. Workspace formatting and all-target Clippy pass. Both wrappers pass:
1,903 functions, zero style violations; zero actionable patterns and 16 existing
reviewed ignores. No ignore/configuration change is made. Final broad wrapper
run passes 626 tests, zero skipped, one slow test, plus workspace doctests;
nextest reports 175.370 seconds, with the expanded inherent-family test at
163.794 seconds. Logs: `/tmp/rstyle-handoff-{nextest.log,clippy.log,style.json,patterns.log}`.

Remaining costs include legitimate pairwise matching, per-unique-record graph
validation/production, optional legacy expansion, repeated source parsing in
existing strict adapters, and JSON materialization of matching-stage details. No
end-to-end speedup is inferred from the bounded helper/graph measurements.


## Borrowed report serialization

The application report now retains borrowed typed unit/function records through
serialization. Its file writer streams through a buffered serializer and checks
flush errors, removing both the full-report `serde_json::Value` conversion and
the extra full serialized file byte buffer. JSON stdout still requires the
string returned by the existing command API. Dynamic matching-stage detail and
optional trace data retain their existing JSON representation. Text output reads
typed counters and outcomes directly. No evidence, completeness indicator,
matching threshold or report field is removed. JSON field order follows the
typed report rather than the former map ordering; field meaning is unchanged.

The shared default regression compares the saved streamed report with parsed
JSON stdout. Existing trace/decision regression passes, and a writer regression
rejects a final flush failure. No separate timing/allocation improvement is
claimed for this small follow-up; its structural reduction follows directly
from eliminating the full report conversion and file-output byte buffer.


Final follow-up validation: `cargo fmt --all -- --check` and workspace/all-target
locked Clippy with warnings denied pass. The nextest wrapper passes 627 tests,
zero skipped, one slow test, in 176.738 seconds; the inherent-family test takes
165.096 seconds and workspace doctests pass. The final default/shared regression
passes after extracting its artifact assertions into a helper. Both final
wrappers pass: 1,905 analyzed functions, zero style violations; zero actionable
pattern candidates and 16 existing reviewed ignores. Logs:
`/tmp/rstyle-final-{nextest.log,clippy.log,shared-test.log,style.json,patterns.log}`.
The earlier too-long test diagnostic was fixed by cohesive assertion extraction;
no check or assertion was removed.
