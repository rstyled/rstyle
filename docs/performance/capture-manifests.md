# Capture-local fallback manifest ownership

Baseline: `d1c7fa8`. Nominal observations in helper profiles without a selected
unit context previously read and SHA-256 hashed Cargo.toml for every observation.
The compiler capture's `Sources` now owns one immutable manifest snapshot per
exact path. Repeated requests reuse the bytes and lazy actual-content identity;
a new capture gets a fresh owner. Failed reads are not cached. Package name,
version, source and declaration association checks are unchanged. No filesystem
metadata authorizes reuse. Valid Cargo manifests are UTF-8; reading the string
preserves its original bytes, including CRLF and BOM.

The regression uses an injected read callback that panics on a repeated read,
checks shared identity storage, observes changed bytes in an independent capture,
and permits retry after failure. This establishes eliminated I/O structurally;
timing alone would not establish it.

Validation: 698/698 nextest tests passed, zero skipped (32.211 s observational),
workspace doctests, formatting and warning-free workspace/all-target Clippy.
Both quality gates passed with zero violations and zero actionable candidates;
the same 15 reviewed ignores remain. No threshold, policy or assertion was weakened.

Measurements use prebuilt debug/release analyzers, pinned nightly-2026-09-30,
aarch64, eight CPU quota, 16 GiB, locked dependencies and sequential commands.
The analyzed fixtures retain ordinary check settings. Prior isolated baseline
samples at `d1c7fa8` were retained rather than rerun after replacing the binaries.
Each workload has a warmup, three ordinary samples and separate diagnostics,
trace on/off, plus strict import replay. GNU time's 0.01 s wall clock and command
peak RSS are coarse proxies. Raw standard-workload samples are in
`capture-manifests-samples.json`; the targeted helper-profile samples are in
`capture-manifests-helper-samples.json`. These are local command observations,
not a whole-suite speedup claim. The standard data-profile workloads generally
already have selected-unit context and do not exercise this fallback.

Standard parity checks passed for 120 complete matching reports and 2,160 frontend
records/options, normalizing only run-directory paths. All strict replays passed.
The 24 study bookkeeping and two measurement-harness Python tests passed.
No-trace medians (seconds) were mixed:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.23 → 0.23 | 0.21 → 0.20 |
| read | 1.11 → 1.08 | 0.31 → 0.31 |
| inherent | 1.52 → 1.55 | 0.32 → 0.35 |
| shared fanout | 0.87 → 0.88 | 0.29 → 0.26 |
| many pairs | 1.53 → 1.54 | 0.32 → 0.35 |
| larger graphs | 3.66 → 3.66 | 0.70 → 0.69 |

Bundle sizes were unchanged. These observations do not establish a runtime gain
for this increment; its supported improvement is eliminating repeated manifest
reads and hashing within one compiler capture.

The targeted inherent-family source under `hir-helpers-v1` completed with unchanged
matching reports, outcomes, exact sources, signatures and completeness. Median
ordinary command time was 0.37 → 0.37 s in debug and 0.24 → 0.24 s in release.
Helper body diagnostic compiler IDs differ when the wrapper binary path changes;
this comparison does not claim byte-identical diagnostic body descriptions. The
standard data-profile 2,160-record comparison above does check exact bodies/options.
An initial helper comparison rejected those expected compiler-ID differences;
its failed attempt is retained in `/tmp/rstyle-helper-manifest-measurement` outputs
only where not superseded by the successful matched runs. No timing threshold is
used as a regression test.
