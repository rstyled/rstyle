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
