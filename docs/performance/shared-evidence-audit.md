# Shared evidence acceptance audit

This audit records revision `d5f3d5f`. The subsequent
[expanded-evidence retirement](expanded-evidence-retirement.md) removes its
optional expansion adapter and updates the current API contract.

This is the consolidated status for the shared-evidence refactor. The earlier
sections of [shared-evidence.md](shared-evidence.md) are a chronological journal;
statements there about pending integration describe those earlier commits.
The format/API contract is [shared-unit-bundle.md](shared-unit-bundle.md).

## Scope and Git provenance

Work is in the `rstyle` repository on `perf/shared-evidence-store`, based on
fetched main `045d4ac331babca8dbc8d33867d7044861ee70a2`. Stabilization prerequisites
were already merged. No unrelated feature history was imported. Commits use
explicit task paths; the user's untracked `slice_access/documents.rs` is
preserved. No style configuration, thresholds or ignore ledger changed.

## Requirement evidence

| Requirement | Implementation and regression evidence |
| --- | --- |
| One owner per helper and paired graph | `unit_store.rs`: moved frontend vector, helper records holding FunctionId/GraphId, roots sharing the same graph. `shared_unit_owns_diamond_helpers_once` checks five unique helpers/graphs and pointer sharing across repeated calls. |
| Mutable construction then immutable analysis | Private Builder owns construction; UnitStore has no public mutation; FunctionView/HelperView resolve references through their owning Resolver. `EvidenceFacts` is sealed. |
| Borrow summaries without full proofs | `helper_calls::verify_use` returns Shared(HelperView); shared summary access borrows; wire emission writes a small helper reference. Legacy owned dependencies are confined to the standalone adapter. |
| No ordinary proof encoding/decoding | Driver capture and CLI use UnitStore; generated helper replay diagnostics record zero legacy verify/transport-encode events. Shared import deserializes graph/helper tables once and moves them into the owner. Typed frontend data tests assert pointer sharing without JSON rendering. |
| Typed local handles and provenance | Private u32 FunctionId/HelperId/GraphId, checked counts/conversions, resolver-bound lookup. Matching has separate local NodeId/EdgeId with checked construction. No public cross-store handle lookup. |
| Persistent identities separate from handles | Exact raw-unit SHA256 includes frontend/build context; existing logical definition identities remain separate. Existing standalone canonical manifests retain their meaning. Unit reordering changes exact identity. |
| Versioned normalized handoff | `unit_store/bundle.rs`, driver output and CLI consume_store: shared tables plus roots, links and certification IDs; `rstyle.shared-unit/1`, separate byte seal. Default shared CLI regression verifies ordinary output contains references rather than expanded artifacts. |
| Verify and decode the same snapshot | capture::load_inventory owns checked bytes; from_current_run verifies the seal and decodes those bytes. `verified_snapshot_survives_file_edits_without_rereading` exercises file replacement after snapshot creation. |
| Trust and policy isolation | from_bundle forces strict policy even within generated scope. Current-run loading explicitly checks independent selected invocation/context. Helper proof and derived-result caches belong to exact resolver and policy. Unit-store, derived and verification-session tests check policy, lifetime and unwind behavior. No serialized cache/capability. |
| Per-use checks retained | verify_shared runs signature/dispatch/inherent-use checks before cache lookup and argument/effect checks for each call afterward. Strict callee verification checks exact summary against verified evidence. Existing helper/read/inherent tampering tests remain. |
| Invalid/cross-unit/contradictory references | Bundle import recomputes call links and identity, validates table bounds, roots/certification inventories, unused graphs and paired agreement. Diamond corruption cases cover wrong kind, dangling links, depths, summaries, missing records and pair disagreement; assert_shared_reference_checks rejects foreign units and inline/reference contradictions under generated policy too. |
| Cycles and budgets | Imported helper depths resolve only certified dependencies within depth eight; unresolved cycles fail. CFG cycles remain legal under existing validation. Existing recursive-helper exclusions and budget tests remain. Large shared fanout regression admits eight functions and all 28 pairs without expanding the closure. |
| Occurrences and operand order | Call links retain HIR occurrence identity. Compact CSR stores edge occurrence IDs, including duplicates/self-edges. Compact adjacency regression and helper argument-position, read-summary and stage tests preserve occurrence/operand semantics. |
| Reordering/renumbering parity | Bundle regression reverses and renumbers helper/graph tables and checks graph meaning/admission. Compact partition/histogram tests and ccgraph hardening/stage fixtures check node navigation/order independence and decisions/scores. |
| Context-qualified types/definitions | No new spelling-only type interner is introduced. Existing nominal, package, target and compiler evidence is retained. Same-source multi-package/modules and nominal inherited-package/target regressions pass through the new handoff. |
| Source correctness and readable views | Borrowed CapturedSources keeps correctness bytes; original frontend source/snippet/expanded provenance remain available. Source ownership tests cover BOM/CRLF and nested owners; validation tests cover byte bounds/digests/missing IDs; CLI expanded-source and transitive-edit tests remain. |
| Bounded optional expansion | `--expanded-evidence` is explicit; expanded.rs enforces depth eight, 8 MiB per call and 512 MiB aggregate work budget, returning errors without truncation. UTF8 budget regression covers byte accounting. Ordinary loading never invokes expansion. |
| Readable reports without eager tree duplication | Report borrows unit facts and matching records, streams file JSON with checked flush; optional WL partitions are typed. Legacy JSON-transform oracle checks status precedence, filtering, unknown fields and partitions. Saved report/stdout parity and trace regressions remain. |
| Borrowed paired documents | PairView threads borrowed PDG/CFG through producer and validators; strict admission retains its frontend decode. Real CLI regression checks pointer identity, wire parity, admission/native-width and malformed CFG behavior under both policies. |
| Quality rules and completeness unchanged | No config/ignore changes; existing threshold, enumeration, gate and slow-family assertions remain. Full wrapper includes doctests. Validation results below distinguish reviewed ignores from actionable findings. |

## Measurements and their limits

Environment: Linux aarch64, eight-core quota, 16 GiB, pinned nightly-2026-09-30
(rustc 5c543b0b8), locked dependencies, unoptimized builds. The journal records
exact workloads, sample logs and instrumentation. Builds/setup are excluded;
ordinary comparisons use alternating samples with matching cache conditions and
no concurrent build/test workload. Instrumented nested stages are not summed.

| Bounded workload | Before | After | Evidence/limit |
| --- | --- | --- | --- |
| 21-function helper certification, 3 alternating samples | 26.354 s wall median; 119,768 KiB RSS median | 1.732 s; 28,060 KiB | Main executable versus unit-owned capture on identical retained snapshot; no whole-CLI claim. |
| Prepared graph, 120 nodes/952 edge occurrences, 5 alternating samples | 76.33 ms median | 5.54 ms | Same graph fixture; RSS 3,688 KiB both. |
| Equivalent evidence storage, 21 functions/20 helpers | 21,650,934 bytes expanded standalone facts+pairs/manifests | 2,985,484 bytes shared bundle | Same evidence/current serializer; separate inventory/seals excluded. |
| Typed frontend data decoding, diagnostic replay | 206 interface / 190 field decodes | 21 / 21 | Before counts lower bounds; diagnostic overhead excluded from timing. |
| Checked lexical evidence, diagnostic replay | 206 nominal / 326 census / 120 inherent checks | 21 / 28 / 7 | Separate instrumented samples; hooks removed. |
| Checked lexical evidence, 3 alternating ordinary samples | 1.691269 s; 27,256 KiB RSS median | 0.568814 s; 25,508 KiB | Same helper replay, not whole CLI. |
| Standalone strict local admission, structural work | 2 full document clones; helper frontend decoded twice | 0 full document clones; frontend decoded once | Code-path and regression evidence; no timing/RSS claim. |
| Readable matching export, structural work | Full RunReport JSON tree plus typed report | Borrowed serialization over typed report | Optional partitions remain typed; no timing/RSS claim. |

No allocation-count or literal zero-copy claim is made. All timings are bounded
local comparisons, not a promise of linear runtime or end-to-end speedup.

## Remaining costs

Legitimate pair enumeration is quadratic. Unique graph production/strict
verification and source/type validation still perform real work. Readable
ImportedFunction/ProjectedGraph adapters retain names, and compact prepared
matching replaces endpoints only on the hot computational path. Frontend build
contexts, snippets and some strings within a single function/pair remain owned;
this work removes multiplied dependency payloads rather than interning every
string. Further source/context dictionaries, binary encoding, compression,
columnar storage and mmap require separate measurements. Optional expanded
exports intentionally incur bounded duplication. JSON stdout retains the
command API's final String. These do not reintroduce nested proof ownership or
ordinary shared import expansion.

## Final validation

The final production revision passes all required commands on this machine:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_TARGET_DIR=target timeout 600 ./scripts/test.sh --nextest
./scripts/style.sh --format json
./scripts/style.sh patterns --fail-on-candidates --show-ignored
```

PATH includes `/home/dev/.cargo/bin`. The test wrapper builds the driver, runs
nextest's CI profile (four test threads), and then workspace doctests, matching
the inspected CI configuration. The full run exits zero: 642 tests passed, one
slow, zero skipped, in 167.312 seconds; doctests also pass. No timeout or partial
analysis is counted as success. Formatting and locked all-target Clippy exit
zero. The focused report compatibility/flush regressions pass too.

The final style JSON contains an empty diagnostics list, 1,994 assessed
functions and zero violations. Patterns completes with zero actionable
candidates and 15 existing reviewed ignored findings. The main baseline had
17 ignored findings; two reviewed patterns disappeared as implementations were
replaced, with no ledger edits. The ignore ledger, rule configuration and
nextest configuration have no diff from the recorded main base. No new
suppression attributes/comments were added. Logs:
`/tmp/rstyle-report-{tests.log,clippy.log,nextest.log,style.json,style.err,patterns.log}`.
The pair-view increment's preceding 641-test validation and both gates are
separately recorded in the journal; no historical result substitutes for this
final run.

No required validation is outstanding. Remaining costs above are explicit
limits of the chosen representation, not missing handoff integration or failed
quality gates.

## Incremental commits

All subjects below are at most 70 characters. Each implementation increment
has its focused validation and gates recorded in the journal. Early partial
status commits explicitly identify their then-incomplete state.

```text
d029c6c Record shared evidence baseline and merged prerequisites
c420fc1 Move completed helper proofs and prepare facts in place
bc5977d Borrow serialized helper documents during verification
28360b6 Store prepared matching graphs with dense handles and adjacency
a02f00d Record partial refactor validation and measured limits
df969a2 Own unit helper evidence and borrow records during analysis
6c26803 Normalize unit graph and helper tables in a versioned bundle
30c19f8 Replace legacy capture with a bounded shared-store export adapter
51c0c33 Pass borrowed evidence through production graph admission
23164f4 Carry shared unit evidence through compiler and CLI handoff
261a06d Stream reports directly from borrowed unit evidence
f54ab9d Borrow captured source buffers during validation and admission
0a94070 Transport profile policies without document JSON round trips
16efbbe Decode data evidence without cloning its JSON tree
bd8e8ae Own typed frontend data evidence and borrow it during analysis
4cbabc6 Borrow checked lexical evidence within each unit analysis
6ba2b70 Borrow paired documents throughout strict admission
24c66d8 Stream matching reports from borrowed typed records
```

The closing documentation commit is `Record the shared evidence acceptance audit`.
No branch is force-pushed and no unrelated commit is amended.
