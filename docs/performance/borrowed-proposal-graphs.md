# Borrowed default proposal graphs

Core clustering used to call `canonicalize` for every input graph, even with the
default policy in which every normalization rule is disabled. That call cloned
all graph nodes, edges, type/signature trees and strings before doing no rewrites.
The reused compiler proposer inherited the same cost as legacy patterns.

Clustering now uses a borrowed normalization view. Disabled normalization returns
the original graph reference; enabled normalization retains the existing owned
rewrite pipeline. The public owned `canonicalize` API keeps its old behavior.
No new cache, hash, body validation, threshold or matching permission is involved.

The diagnostic runs the complete `patterns_report::run_owned` pipeline on the
same retained rstyle-ccgraph library facts: 238 functions/graphs, 3,847 nodes and
4,858 edges. It compares baseline `a1edd6a` against this source change using
unoptimized standalone probe binaries and the same offline-resolved dependency
versions. These temporary manifests do not use the workspace lockfile; the
checkpoint gates do. Both probes perform one warmup and seven measured runs.
A temporary `System` allocator wrapper counts allocations/reallocations and their
requested sizes only during proposal evaluation. Input decoding/cloning, report
comparison and output serialization are outside the measured region.

| Full proposal pipeline | Before | After |
| --- | ---: | ---: |
| Allocation/reallocation calls per run | 183,312 | 166,555 |
| Cumulative requested bytes per run | 12,333,551 | 10,469,752 |
| Median instrumented time, milliseconds | 108.667 | 108.838 |

The report is exactly equal, including candidate identities, scores, membership,
signature groups and suppressions. Allocation calls fall by 9.1% and requested
bytes by 15.1%. Requested bytes are allocation traffic, not peak RSS. The timing
is effectively unchanged; no speedup is claimed. This measures saved-fact core
proposal evaluation, not compiler capture, strict matching, native macOS or
whole-workspace performance. The existing rational-arithmetic proposal remains
a known rejected suggestion; this optimization does not improve proposal quality.

[Samples and retained diagnostic locations](borrowed-proposal-graphs-samples.json)
record all runs and the report. Instrumentation stays outside production code.
Regression tests require the default view to point to the original graph and
enabled rewrites to preserve the input. Existing normalization, clustering,
proposal and compiler-adapter acceptance tests cover behavior.
