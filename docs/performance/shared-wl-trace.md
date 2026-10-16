# Shared scoring and trace refinement

Clone analysis now collects optional WL label partitions during the scoring
refinement. It no longer looks up each pair's functions again, reprojects both
graphs, rebuilds compact adjacency, and repeats all refinement rounds in a
separate report pass. Node names are prepared once per projected graph when
trace output is requested. Numeric labels and the interner remain pair-local;
no pair-dependent label is cached as graph-global evidence.

`evaluate_selected_with_options` returns the ordinary `RunReport` and optional
partitions indexed by its exact pair position. Existing matching entry points
retain their report-only behavior. Normal evaluation uses the same refinement
with a no-op observer and creates neither trace-name vectors nor partition
entries. Trace observations use the exact synchronous labels that produce the
scoring histograms. Partitions are retained only after scoring succeeds, matching
the previous CLI behavior of tracing successfully scored pairs. Resource-limited
runs still stop before enumeration.

Report fields, score detail bytes and trace partition names are unchanged.
The diagnostic stage `cli.trace_partitions` disappears: that work is now included
in `cli.matching`, so those two stages must not be compared independently across
revisions. Report publication still checks final flush and preserves the same
exit status rules.

Focused regressions compare trace and normal reports, integrated and standalone
partitions, and partition-derived versus scoring histograms with parallel edges
and self-edges. They cover both overlap and cosine scoring, along with empty
partition output for ordinary and pair-budget-limited runs.

## Measurements and validation

Baseline source is `2a19b63`, using the retained previous increment's isolated
samples and saved binaries in `/tmp/rstyle-before-shared-wl-trace`. Candidate is
this commit. [312 command samples](shared-wl-trace-samples.json) cover six
workloads, debug/release analyzers, trace off/on and strict import replay.
Fixtures, source paths, warm cache conditions and harness commands match the
previous increment. Setup/builds are excluded; timings ran without concurrent
builds, tests or report comparisons. Each setting has one warmup, three ordinary
samples and one separate diagnostic sample. Aarch64, eight-CPU quota, 16 GiB,
nightly-2026-09-30 and locked dependencies; the analyzed fixture stays in the
ordinary check profile. GNU time resolution is 0.01 seconds; RSS is whole-command
peak memory, not attribution to partition storage.

Median ordinary trace-enabled wall seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.23 → 0.23 | 0.21 → 0.21 |
| read | 1.16 → 1.17 | 0.32 → 0.29 |
| inherent | 1.68 → 1.70 | 0.38 → 0.34 |
| shared-fanout | 0.94 → 0.95 | 0.28 → 0.29 |
| many-pairs | 1.85 → 1.79 | 0.36 → 0.38 |
| larger-graphs | 4.42 → 4.14 | 0.74 → 0.72 |

No-trace larger-graph medians were 3.64 → 3.66 seconds debug and 0.72 → 0.71
release. No uniform speedup is established. The separate larger-graph diagnostic
run attributes debug matching plus trace from 323.07 + 299.77 ms to 351.00 ms;
release from 33.94 + 32.31 ms to 41.86 ms. These old stages were sequential,
not nested. The new matching stage includes partition collection.

Structurally, each traced WL pair avoids two projections, two compact adjacency
builds and one complete second refinement. Existing scoring histograms and output
partitions remain; node-name preparation happens once per projected graph.
No-trace runs do not collect partitions. All 120 complete matching reports compare
exactly, including detail strings and partitions. Harness score/decision,
admission, completeness and strict replay assertions pass.

Commands used the existing `measure_library_resolution.py` and
`measure_broad_pipeline.py` harnesses with `after target`, writing to
`/tmp/rstyle-fast-checksum-measurement` and `/tmp/rstyle-fast-checksum-broad`.
All 678 nextest tests passed, zero skipped; workspace doctests passed. Formatting,
warning-free workspace/all-target Clippy, 24 study tests and two harness tests
passed. Both required gates report zero violations and zero actionable pattern
candidates; 15 reviewed ignores remain unchanged. Logs are
`/tmp/rstyle-shared-wl-trace-*`.
