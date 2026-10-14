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
