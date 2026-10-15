# Remaining production costs

The optimized tool's largest measured phase on these workloads is compiler
capture, followed by handoff and evidence verification. Hashing dominates the
debug tool's runtime but is small in release. Matching, trace partitions and
report serialization are not the first optimization targets at this scale.

This is a profile of production code paths, not an optimization or a before/after
speedup claim. Base: `69be5c6`. The implementation adds opt-in diagnostic stage
boundaries and a strict-import example; it changes no admission or matching
policy, representation, identity, evidence budget or normal report fields.

## Workloads and method

Linux aarch64, eight-CPU quota (`800000 100000`), 16 GiB cgroup memory,
nightly-2026-09-30, rustc commit `5c543b0b8c73c7b72bc8284ced4fb22ead15734d`,
locked workspace dependencies. The existing debug and release profiles are
unchanged. Tool build time is excluded; release workspace build took 21.18 s.
Each tool profile uses its matching driver. Fixture compilation uses the same
ordinary Cargo check profile in both cases; the clones `--release` option is
not used. Release here means optimized analyzer/driver binaries.

The minimal workload has two scalar functions/one pair. Read-helper and
inherent-helper workloads use the exact checked-in `DATA_READ_CALL_FAMILY` and
`DATA_INHERENT_FAMILY` constants in the CLI tests: 20/21 admitted functions and
190/210 exhaustive pairs. The latter two accept 13/11 pairs respectively.

Each workload starts in a new dependency-free fixture with an offline-generated
lockfile. One first-capture run is retained separately. Then three uninstrumented
plain and trace runs alternate in the same fixture, with warm filesystem/build
state. Warm commands still recapture evidence; this is not a cached assessment.
One separate diagnostic command per mode uses `RSTYLE_PERF=1`. Finally the last
captured bundle undergoes three ordinary strict imports and one diagnostic
strict import. Each command is bounded by `timeout 120`; no concurrent builds,
tests or profiling commands run. `/usr/bin/time` records wall time and peak RSS.
Its 0.01-second wall resolution cannot resolve the smallest import timings.
RSS includes child-process accounting and is not summed across processes.

Every CLI report must have all four completeness flags, expected admitted
function/pair counts, and identical decisions and displayed scores across its
runs and trace settings. Debug/release decisions and scores also agree for all
401 pairs after matching unordered pairs by original function names; run-local
keys and pair enumeration order can differ with provenance. Strict replay must
complete and retain the expected function/graph counts. It does not rerun
capture or matching and explicitly reports `source_to_pair_complete: false`.

## Ordinary measurements

Medians of three successful warm runs. These compare build profiles, not code
versions; they do not establish a refactor speedup.

| Workload | Debug plain | Debug trace | Release plain | Release trace |
| --- | ---: | ---: | ---: | ---: |
| Minimal | 2.17 s | 2.18 s | 0.22 s | 0.22 s |
| Read helpers | 3.90 s | 3.93 s | 0.43 s | 0.43 s |
| Inherent helpers | 4.60 s | 4.57 s | 0.47 s | 0.49 s |

| Strict captured-bundle import | Debug | Release | Bundle bytes, debug/release |
| --- | ---: | ---: | ---: |
| Minimal | 0.01 s | below 0.01 s resolution | See raw samples |
| Read helpers | 0.68 s | 0.07 s | 2,036,579 / 2,036,660 |
| Inherent helpers | 1.02 s | 0.09 s | 3,043,531 / 3,044,290 |

Bundles differ in producer/build provenance. Byte sizes are not a storage
optimization comparison. Median plain-command RSS for read/inherent workloads
is 113,464/120,764 KiB in debug and 109,232/116,516 KiB in release. Standalone
release strict-import RSS is 20,100/29,396 KiB. The compiler remains part of
CLI-command memory accounting.

## Diagnostic attribution

Seconds from a single separate plain diagnostic run per workload/profile.
These are inclusive wall observations, not CPU samples. Diagnostics are not
used for the ordinary timing medians above. Child-stage logging, scheduling
and instrumentation can perturb observations.

| Phase | Debug read | Debug inherent | Release read | Release inherent |
| --- | ---: | ---: | ---: | ---: |
| Prepare | 1.983 | 1.973 | 0.053 | 0.055 |
| Binary hashes, **inside prepare** | 1.941 | 1.934 | 0.017 | 0.015 |
| Compiler/capture | 1.035 | 1.347 | 0.191 | 0.252 |
| Unit capture, **inside compiler/capture** | 0.317 | 0.500 | 0.033 | 0.057 |
| Handoff | 0.527 | 0.719 | 0.080 | 0.077 |
| Matching | 0.204 | 0.325 | 0.009 | 0.014 |
| Write saved report | 0.046 | 0.062 | 0.002 | 0.004 |
| Render stdout JSON | 0.054 | 0.072 | 0.002 | 0.003 |

Release handoff includes one bundle decode (11–12 ms), independent selected-unit
inspection (11–16 ms), structural validation (1–2 ms) and generated-policy
verification (22–29 ms). Its remaining time includes source inventory,
admission/projection and application bookkeeping. Diagnostic strict-import
verification instead takes 49/87 ms for read/inherent bundles, versus decode
7/16 ms and structural validation 2/4 ms. Generated and strict verification are
different authority paths and must not be treated as interchangeable caches.

Trace partitions take about 1 ms in the release helper workloads. Reports grow
from 1.73 to 1.96 MB for read helpers and 2.43 to 2.65 MB for inherent helpers.
The stdout-render boundary excludes writing that returned String to the actual
stdout sink. Process startup and other code outside the observed boundaries
also remain unattributed. Nested stage times must not be added as independent
costs, and these stage samples must not be subtracted from ordinary medians.

## Next investigation

1. Split compiler capture into Cargo/rustc orchestration, HIR extraction,
   per-unit certification and evidence publication. The broad phase is largest
   in release; the 33/57 ms unit-capture substage explains only part of it.
   Do not assume the rest is subprocess overhead or skip required recapture.
2. Attribute strict verification to source/type, nominal and graph checks.
   Verification dominates strict import; eliminating JSON decoding alone would
   miss most of that workload. Preserve policy and owning-context isolation.
3. Measure larger corpora before more matching changes. These samples cover
   at most 210 small-function pairs, not large graph/fanout scaling. Pair
   enumeration remains exhaustive, and current data does not justify new
   compression, dictionaries or binary formats.

Debug hashing is still a test/developer cost, not the largest release cost:
release binaries are about 14 MiB/8 MiB versus 157 MiB/175 MiB in debug.
No metadata-only executable identity cache is proposed.

## Reproduce and artifacts

Build first, then run the script sequentially:

```sh
cargo build --workspace --locked
cargo build -p rstyle-pdg --example unit_import_replay --locked
python3 scripts/profile_clones.py --profile debug
cargo build --release --workspace --locked
cargo build --release -p rstyle-pdg --example unit_import_replay --locked
python3 scripts/profile_clones.py --profile release
```

The script requires GNU `time`, `timeout`, Python 3 and the pinned toolchain on
PATH. It prints a fresh artifact directory and retains full reports, stderr,
timing files and `samples.json`, including failures. Existing source fixtures
are read, never modified. New diagnostic boundaries have no evidence hashing
argument, so enabling them does not rehash bundles just to identify a stage.
A stage finishing means its operation returned; report completeness remains the
independent authority for whole-run success.

[Machine-readable results](production-profile-results.json) retain every sample
and aggregated stage visit counts/durations. Original directories are
`/tmp/rstyle-production-profile-s503tesn` (debug) and
`/tmp/rstyle-production-profile-utnfx2qj` (release). Console logs are
`/tmp/rstyle-production-profile-console.log` and
`/tmp/rstyle-production-profile-release-console.log`. The debug samples used
the equivalent original `/tmp/rstyle-production-profile.py`; the checked-in
script adds profile selection and formatting. No failed/timeout command is
counted as a successful sample.

## Validation

Both tool profiles built with locked dependencies. All 54 CLI runs and 24
strict-import replays completed successfully. The final full suite passed:
642 tests, zero skipped, 106.083 seconds, no slow warnings, plus workspace
doctests. Commands actually run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_TARGET_DIR=target timeout 600 ./scripts/test.sh --nextest
./scripts/style.sh --format json
./scripts/style.sh patterns --fail-on-candidates --show-ignored
```

The style report contains 1,970 functions and zero violations; patterns reports
zero actionable candidates and 15 existing reviewed ignores. No configuration,
threshold, suppression, ignore ledger or concurrency setting changed. Final
logs use `/tmp/rstyle-prod-profile-nextest.log`,
`/tmp/rstyle-prod-profile-final-{fmt,clippy,patterns}.log`,
`/tmp/rstyle-prod-profile-final-style.json` and
`/tmp/rstyle-prod-profile-final-style.err`. Diagnostic behavior and normal
report decisions were checked by the separate profiling runs, including
instrumentation on/off and strict imported bundles. Profiling records and
reports are not substituted for these quality gates.
