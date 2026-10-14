# Clone regression investigation

Work starts from main `7432a5a` on `perf/clone-analysis-stabilization`.
The original family assertions and initial/warm/edit analyses are retained.
Capability expansion and the broad analysis-library extraction remain separate
from this stabilization branch. Measured fixes address ARM executable hashing,
repeated generated-evidence reconstruction, immutable transport encoding and
recursive callee verification. The restored full-family test passes in 235.048s,
including strict post-return checks, trace, warm parity and edit propagation.
The full 614-test workspace passes in 246.948s of nextest execution with four
threads. Both default rstyle gates are clean. A comparable known-good historical source
baseline is unavailable; the sections below preserve investigation milestones.

## Retained historical evidence

User-supplied ARM nextest log for `8e94840`: 604 passed, 20 slow, 0 skipped,
6615.776s execution wall time. Ranked cases:

| Case | Duration (s) |
| --- | ---: |
| Data-v3 inherent family | 6521.289 |
| Data-v2 complete shared field/helper families | 947.138 |
| Data-v1 shared results/all pairs | 462.718 |
| Fresh/warm trace decisions | 219.871 |
| Inherent all pairs/separate selection | 192.921 |
| Simple clone cases | about 22–25 |

These are overlapping test durations, not stage CPU attribution. The historical
13-minute baseline lacks a matching commit/runner/workload. Runner change
`6c0749c` and capability change `8e94840` still need independent comparisons.

## First bounded observation

Environment: Linux aarch64 sandbox (runner size not established), pinned
`nightly-2026-09-30`, rustc `5c543b0b8`, unoptimized dev/test profiles, warm workspace
build artifacts, fresh compiler-capture target, one CLI command, no test concurrency.
Source has three scalar functions: `leaf(x) = x`, `wrap(x) = leaf(x)`,
`third(x) = wrap(x)`, selected with `hir-helpers-v1`, trace off. This is a new
small diagnostic fixture, not an unchanged-test regression comparison.

With opt-in detailed diagnostics and no algorithm fix:

| Observation | Value |
| --- | ---: |
| Supervisor elapsed | 21.899s |
| Driver binary read + SHA-256 | 7.990s |
| CLI binary read + SHA-256 | 7.020s |
| Cargo compilation (includes driver production) | 4.305s |
| Helper verification start events | 143 |
| Producer verification start events | 161 |
| Maximum visits to one exact helper-evidence digest in one thread | 72 |

Binary sizes: driver 182,586,368 bytes, CLI 158,424,672 bytes. Thus hashing is a
measured contributor to this sandbox's setup floor. Counts demonstrate repeated
work but do not establish a complexity class or the full-family root cause.
Stage durations nest and must not be summed as independent costs. Diagnostic
hashing/logging adds overhead; use uninstrumented runs for performance comparisons.
Both pair enumeration and evaluation were complete.

Session artifacts: `/tmp/rstyle-perf-chain-2-observed/{run.json,stdout.log,stderr.log}`;
source/capture under `/tmp/rstyle-perf-chain-2`. They are sandbox-local, not portable
checked-in replay bundles. At this initial milestone, facts snapshots and explicit replay were still pending.

## Validation so far

Workspace builds; watchdog cleanup/output tests, diagnostic evidence-count test,
six PDG `data_types` tests and four CCGraph `data_matching` tests passed.
CLI target compiles. Exact streaming-adapter output/status test passed.
Both default `cargo rstyle --format json` and
`cargo rstyle patterns --format json --fail-on-candidates` returned zero after
instrumentation. No thresholds or ignore entries were changed.

At this initial instrumentation milestone, full-family E2E, warm/edit analysis,
old-revision comparisons and broad acceptance had not run. Subsequent sections
record fixes and completed validation; this milestone alone established no speedup.

## Measured ARM setup fix

The existing zero-admitted-functions test
`clones_unsupported_workspace_completes_assessment_with_zero_admitted_functions`
executes one CLI analysis. Before the backend change it passed in 14.90s:
7.861s driver hashing, 6.828s CLI hashing, 0.094s compiler capture.
After it passed in 2.42s: 1.086s driver hashing, 1.034s CLI hashing, 0.143s capture.
Both measurements used the same sandbox, pinned toolchain, original dev/test
profiles, warm build artifacts, fresh compiler capture, exact filter and
`RSTYLE_PERF=1`; no whole-suite timing claim follows from this comparison.

SHA-2 0.10.9 selects its ARM runtime-detected SHA-256 backend only with the `asm`
feature. The fix enables that existing feature for cargo-rstyle on aarch64, with
its software fallback retained. Other architectures retain the existing feature
selection. No build-profile optimization, cache, identity rule or verification
check changes. ARM builds now compile the upstream assembly support dependency
and need a working target C/assembler toolchain; this sandbox build passed.
Cargo may unify the SHA-2 feature across workspace packages built together.

The three-function diagnostic completed in 7.117s after the fix, versus 21.899s
before. Combined binary hashing: 2.020s versus 15.010s. Compilation including
production: 3.244s versus 4.305s. Counts remain 143 helper verifications and 161
producer verifications. All three functions remain admitted; all three pair
results and gate details agree after accounting for pair orientation caused by
rebuilt-binary identities. Rebuilt executable digests naturally differ; no hashed
evidence was rewritten or identity rule relaxed. Python `hashlib` independently
verified both new executable hashes. Empty, `abc` and million-`a` SHA-256 vectors
also pass through the real file-digest path, including edits to the same file.

Retained after-run logs: `/tmp/rstyle-perf-chain-2-arm-backend` and
`/tmp/rstyle-perf-cli-floor-arm-1`. Source-handoff CLI correctness also passes
without diagnostics (2.37s). This fixes a measured setup contributor; attribution
of the historical source regression and the long helper-family tail remains open.

## Bounded helper-growth reproduction and raw-family capture

Under the same ARM backend, adding `fourth(x) = third(x)` raises the scalar-chain
observation from 7.117s to 42.016s. Helper visits: 143 to 510; producer verifications:
161 to 532; cumulative encoded helper evidence bytes across visits: 18,679,614 to
181,565,333. Largest call evidence: 447,010 to 3,981,068 bytes. These are two measured
points with detailed diagnostic overhead, not an established complexity class.
The four-function run completes all six pairs; its logs/source remain under
`/tmp/rstyle-perf-chain-3-observed` and `/tmp/rstyle-perf-chain-3`.

The exact motivating source was copied from `DATA_INHERENT_FAMILY`, including its
BOM and CRLF, into `/tmp/rstyle-perf-inherent-family`. Capture-only completed raw
HIR/unit facts for all 21 functions in less than one second of compiler capture.
It did not run helper certification or publish normal documents; the CLI returned
incomplete capture, as required. Version-1 pre-certification snapshot:
`/tmp/rstyle-perf-inherent-raw/1595339-pre-helper-certification.json` (830,724 bytes).
This is a diagnostic snapshot, not a complete captured evidence bundle.

The standalone `helper_replay` executable loads that snapshot in one process
(load 0.021s) and calls the same ordinary-library helper certification path.
It timed out at the declared 120-second bound; the supervisor cleaned up its
process group and retained input/logs at
`/tmp/rstyle-perf-inherent-helper-replay-baseline`. It repeatedly verified identical
`Data::get` evidence (over 320 visits) and encoded that dependency again (over 130
visits). This identifies a shared recursive path independently of compiler work;
no successful full-family replay or semantic test pass is claimed for the timeout.

The user subsequently revised the threat model: locally generated graphs are not
adversarial, and costly tamper-proofing is unnecessary. Subsequent fixes should
trust current-run driver production and avoid redundant reconstruction while
retaining structural graph checks and independent correctness coverage. Imported
replay remains explicitly distinguishable from current-run production.

## Current-run generated evidence policy

Following the user's revised nonadversarial threat model, the driver and fresh
CLI handoff explicitly enter a trusted-generated scope. Producer verification
keeps structural validation (including paired correspondence and value sites)
but does not regenerate an expected graph from the same compiler facts. Helper
use still checks caller signatures, inherent dispatch, dependency boundaries and
arguments, without recursively recomputing the already generated callee summary.
Document digests used for identity/byte correspondence remain intact.
Independent library calls and imported replay default to the prior strict path.
Policy is thread-local and is restored on nesting, early return and panic.
The report and diagnostic events disclose the chosen policy.

The exact raw 21-function family helper replay with `--generated` completed in
37.788s (load 0.033s, helper capture 37.729s), versus a strict timeout at 120s.
It processed 157 helper-verification visits, 154 producer verifications, 66
transport encodes and 24 production attempts. Transport encodes alone total
13.670s; repeated encoding of immutable certified dependencies is the next
measured opportunity. Results/logs:
`/tmp/rstyle-perf-inherent-helper-replay-generated`. This replay certifies the
helper stage only; it does not establish full production/matching acceptance.

The scoped-policy unwind/thread test passes. A focused real CLI test verifies
strict/generated production yields byte-identical CFG/PDG and manifests, strict
verification still accepts that generated output, and generated policy still
rejects duplicate graph node IDs. A concurrent-build CLI attempt failed while
fingerprinting a replaced executable and never reached analysis; it is recorded
as setup failure, not an algorithm timeout. Subsequent measurements use stable
copies of the matching sandbox binaries.

One complete traced full-family CLI analysis now finishes under its 120s watchdog:
73.045s total, 63.159s compiler capture/production, 39.304s helper certification.
It admits all 21 functions, evaluates all 210 pairs, and retains all four
completeness flags as true. Encodes total 18.478s across driver/CLI work; producer
production spans total 33.060s (nested within capture, not additive). Logs, report
and both phase snapshots are retained under
`/tmp/rstyle-perf-inherent-generated-cli-stable` and
`/tmp/rstyle-perf-inherent-generated-facts-stable`. This is one initial traced
analysis, not the complete three-analysis restored E2E test or suite acceptance.

## Reuse immutable helper transport within capture

Each capture now owns a certified dependency together with its lazily initialized
transport encoding. Attachment clones the already encoded value; it does not
serialize and rebuild the lossless table for every caller. The cache ends with
that capture and is tied to the exact immutable dependency, rather than relying
on a function name across runs. Encoding remains lazy so unused dependencies do
not newly fail transport budgets. Per-call description size/depth checks remain.

On the same raw snapshot, generated helper replay completes in 32.193s versus
37.788s; transport encoding falls from 66 calls / 13.670s to 31 / 7.881s.
One traced full-family CLI analysis completes in 67.457s versus 73.045s, with
21 admitted functions, 210 pairs, and all four completeness flags true. All
210 decisions, displayed scores and evaluation statuses agree after mapping
rebuilt function keys to names. Retained artifacts:
`/tmp/rstyle-perf-inherent-helper-replay-encoded-once` and
`/tmp/rstyle-perf-inherent-encoded-cli`. These single observations are not a
statistical benchmark or acceptance of the restored three-analysis test.

Validation: six data-type tests, five transport/lexical tests, changed-target
Clippy with warnings denied, formatting, and both default rstyle gates at zero.

## Restored test phases

The original inherent-family test now emits separate initial, post-return
assertion/tampering, warm, and edit phase events without removing assertions or
follow-up runs. A 120s diagnostic run completed the initial CLI in 67.452s and
then timed out during strict post-return verification. Identical `Data::get`
evidence had been revisited at least 139 times; its transport was being encoded
for the 57th time when terminated. Warm/edit phases were not reached. Artifacts:
`/tmp/rstyle-perf-inherent-restored-phases-ready`. The process group was terminated;
this is incomplete validation, not a semantic test failure or acceptance pass.
An earlier direct-test launch lacked `CARGO_BIN_EXE_cargo-rstyle` and failed
immediately; the retained setup-failure logs are separate from this observation.

## Bounded successful-callee reuse and restored acceptance

Production, capture, and root verification now share a thread-local session of
successful callee proofs. A key contains exact complete resolution bytes
(including dependency documents, manifest, facts and build context), partitioned
by strict/generated policy. No new hashing is performed. Caller signature,
inherent dispatch, depth and argument checks still run for every use; only an
already successful callee proof can be reused. Errors are not cached. Each root
operation discards the session, including on unwind, and retained key bytes are
bounded at 32 MiB. Reaching that budget disables further retention, not validation.

A first experiment scoped only individual verifier calls remained insufficient:
strict full-family helper replay timed out at 120s. It is retained separately as
`/tmp/rstyle-perf-inherent-helper-replay-session-strict`. Extending the lifetime to
one whole capture/production operation also covers repeated eligibility/lowering
and emission work, without retaining evidence across independent calls/runs.

The **restored original full-family E2E test passes**, including all original
21-function/210-pair, helper/data, tampering, warm parity and edited-source checks.
Its declared 300s runtime budget completed in **235.048s**:

| Test phase | Elapsed |
| --- | ---: |
| Initial traced CLI | 64.941s |
| Post-return verification and tampering | 41.138s |
| Fresh warm CLI and parity | 64.485s |
| Edited-source recapture and propagation | 64.390s |

The test process records 145 helper verifications, 75 reused callee proofs,
70 producer verifications and 52 transport encodes. Retained logs and watchdog
metadata: `/tmp/rstyle-perf-inherent-restored-session`. The historical supplied
ARM duration was 6521.289s, but runner/build differences prevent treating that
comparison as a controlled speedup. The directly preceding local run exceeded
120s during assertions after its 67.452s initial analysis; these observations
isolate a measurable improvement without pretending the timeout was a pass.

Validation includes all 40 PDG unit tests, session lifetime/policy/thread/unwind
and retention-budget checks, and a repeated-callee CLI regression: strict
production agrees byte-for-byte with current-run documents/manifests and still
rejects changed manifest, source, signature and effects. Changed-target Clippy
with warnings denied and both default rstyle gates are clean. Older representative
families and the broader suite remain separate validation work.


## Representative cases after stabilization

All original cases below pass independently under 120s watchdogs, using the
same stable copies of matching debug binaries and a warm workspace build cache.
Compiler capture still creates fresh targets. Retained results/logs are under
`/tmp/rstyle-perf-representative-{helpers,data-v1,data-v2,trace-parity,setup-floor}`.

| Unchanged case | Current local test (s) | Supplied historical ARM CI (s) |
| --- | ---: | ---: |
| Scalar acyclic helper family | 4.072 | Not supplied |
| Data-v1 shared results/all pairs | 22.872 | 462.718 |
| Data-v2 complete shared field/helper family | 78.504 | 947.138 |
| Fresh/warm trace decisions | 22.321 | 219.871 |
| Unsupported-workspace setup floor | 2.231 | Simple-case category about 22–25 |
| Restored Data-v3 inherent family (separate 300s budget) | 235.048 | 6521.289 |

Historical/current columns are not a controlled speedup comparison: the old
runner size and build-cache state are unavailable. The earlier local hashing and
helper-replay comparisons isolate individual changes; this table establishes
current correctness/runtime for older shared-path cases as well as Data-v3.
Some focused runs overlapped workspace compilation, so their timings are single
observations with scheduler noise, not calibrated regression budgets.

Current environment: Linux aarch64; 20 visible heterogeneous ARM cores
(Cortex-X925/Cortex-A725), cgroup CPU quota 8 cores (`800000/100000`), memory limit
16 GiB. Pinned rustc `5c543b0b8c73c7b72bc8284ced4fb22ead15734d`, nightly
2026-09-30, unoptimized dev/test profiles. Branch base is remote main
`7432a5ab0a4b9de90f23e5ccb159487ee7d0d360`; implementation revision `0375418`.
The standalone full-family phase run enables diagnostic logging. Representative
runs disable it. Full workspace acceptance uses nextest's existing `ci` profile
with four test threads and a separate 600s supervisor budget; its inventory is
614 tests, versus 604 in the supplied historical log. Additional coverage is
reported separately rather than pretending those workloads are identical.

## Full workspace acceptance

The full workspace nextest run at implementation revision `0375418` passes:
**614 passed, 2 slow, 0 skipped**, across 23 test binaries, with the unchanged
four-thread `ci` profile. Nextest execution wall time is **246.948s**; the external
supervisor records **249.584s**, including 2.41s of final test-target compilation
(the test inventory had already been built separately). Summed overlapping test
durations are **718.102s**; that sum is not suite wall time or CPU attribution.
The restored full-family case passes again under concurrent suite scheduling in
235.640s. The other slow case, Data-v2, passes in 77.601s. Ranked following cases
include Data-v1 23.034s, trace parity 22.587s and inherent separate selection
13.488s. The entire run fits its declared 600s supervisor budget.

Logs/metadata: `/tmp/rstyle-perf-workspace-nextest`. The workspace doctest command
also passes (no current doctests), retained under
`/tmp/rstyle-perf-workspace-doctests`. All three watchdog behavioral tests pass,
including timeout descendant cleanup. Final formatting and both default rstyle
quality gates are clean; thresholds, ignore entries, trace coverage, profile
budgets and original test assertions were not relaxed.

This establishes performance stabilization and full existing coverage in this
sandbox. It does not establish a controlled historical CI speedup or implement
the proposed `rstyle-analysis` crate, complete portable evidence bundles,
selected-pair APIs or comprehensive replay migration. Those remain separate
architecture work after this measured stabilization.
