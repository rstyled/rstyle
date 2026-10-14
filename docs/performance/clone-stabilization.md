# Clone regression investigation

Work starts from main `7432a5a` on `perf/clone-analysis-stabilization`.
The original family assertions and initial/warm/edit analyses are retained.
Capability expansion and the broad analysis-library extraction are deferred while
bounded measurements identify shared-path costs. No completed full-family stage
profile or comparable known-good source baseline is available yet.

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
checked-in replay bundles. Facts snapshots and explicit replay remain pending.

## Validation so far

Workspace builds; watchdog cleanup/output tests, diagnostic evidence-count test,
six PDG `data_types` tests and four CCGraph `data_matching` tests passed.
CLI target compiles. Exact streaming-adapter output/status test passed.
Both default `cargo rstyle --format json` and
`cargo rstyle patterns --format json --fail-on-candidates` returned zero after
instrumentation. No thresholds or ignore entries were changed.

The full-family E2E test, its warm/edit analyses, a comparable old-revision
baseline and broad suite acceptance have not run. No performance improvement or
full proposal completion is claimed by this initial instrumentation milestone.

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
