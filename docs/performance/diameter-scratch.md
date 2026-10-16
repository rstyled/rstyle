# Diameter traversal scratch experiment

The retained production diameter traversal is unchanged. A candidate reused one
`seen` vector and one node-capacity queue across every start vertex in a graph,
resetting the flags, cursor and maximum before each BFS. This would replace
per-start allocations with two per graph, without changing direction,
reachability, self-edge or parallel-occurrence semantics. The queue's reserved
capacity also increases storage for mostly disconnected traversals.

The candidate is rejected for now: optimized larger cases improved, but debug
and small optimized cases regressed. Existing label-buffer rotation, integer
histograms and scoring/trace sharing remain independent retained improvements.

The prebuilt `diameter_bench` example constructs 32/128/512-node directed chains
and fanout graphs with three parallel edges per connection and a self-edge.
Fixture creation is outside timing. Every invocation checks the exact expected
diameter; each sample contains 100 public API calls, including unchanged compact
graph preparation. One unmeasured assertion warms each case, then five ordinary
samples run without diagnostics. Baseline is `330b7d2`; the candidate is the
local scratch-reset experiment described above, not a committed production
change. Builds and validation did not overlap measurements. Environment:
aarch64, eight-CPU quota, 16 GiB, pinned nightly-2026-09-30, locked dependencies.

Median seconds per 100 calls (before → candidate):

| Nodes | Shape | Debug | Release |
| --- | --- | --- | --- |
| 32 | chain | 0.01287 → 0.01264 | 0.00108 → 0.00149 |
| 32 | fanout | 0.00704 → 0.00725 | 0.00045 → 0.00088 |
| 128 | chain | 0.11938 → 0.12347 | 0.00883 → 0.00756 |
| 128 | fanout | 0.03527 → 0.04000 | 0.00234 → 0.00225 |
| 512 | chain | 1.45433 → 1.54254 | 0.10307 → 0.09119 |
| 512 | fanout | 0.14728 → 0.23472 | 0.01079 → 0.01030 |

[Raw samples](diameter-scratch-samples.json) retain all five observations. These
are microbenchmark results, not complete-pipeline timings or allocation-counter
measurements. Particularly small cases are sensitive to environment drift;
reconsideration should use interleaved saved binaries and allocation attribution.
The mixed evidence does not justify changing the default traversal here.

Retained-code validation: 692 nextest tests passed with zero skipped, workspace
doctests passed, formatting and warning-free workspace/all-target Clippy passed.
Both quality gates report zero violations/actionable candidates, with the same
15 reviewed ignores. Logs: `/tmp/rstyle-diameter-*`. The candidate itself was
measured through exact expected-diameter assertions, not claimed fully validated.
