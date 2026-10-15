# Pair-local refinement label buffers

Synchronous strict WL refinement now rotates two label vectors per graph. The
seed labels are the first current buffer; the first round allocates the second.
Later rounds clear and refill the inactive buffer while reading the complete
previous round, then exchange the buffers. Each graph keeps its own buffers;
there is no cross-pair state or graph-global label vocabulary.

For a nonempty graph refined through depth `h`, label-vector allocations fall
from `h + 1` to `min(h + 1, 2)`. Seed-only and one-round comparisons have no
allocation reduction. Empty graphs allocate no label storage. This count covers
label vectors only: complete interner tuple keys, neighbor vectors, histogram
trees and output partitions remain. Retaining both vectors does not remove the
need for all round histograms required by the scoring API.

Shared full-key interning, node traversal order, direction, edge family,
multiplicity, self-edges, synchronous rounds and scoring are unchanged. A focused
structural regression checks both graphs' allocation addresses across buffer
rotation, distinct previous/current labels, and complete round histograms and
interner counts. Existing graph and scoring regressions remain enabled.

## Measurements and validation

Baseline source is `f2fbf16`, using the previous increment's isolated retained
samples, with saved tools in `/tmp/rstyle-before-wl-scratch/{debug,release}`.
Candidate is this commit. [312 raw samples](wl-label-buffer-samples.json) cover
six fixtures, debug/release analyzers, trace modes and strict replay. The existing
`measure_library_resolution.py` and `measure_broad_pipeline.py` harnesses used
`after target` and the same `/tmp/rstyle-fast-checksum-{measurement,broad}` fixture
roots. The fixture remains in the ordinary check profile. Builds/setup, tests and
report comparisons did not overlap timing. There is one warmup, three ordinary
samples and one separate diagnostic sample per setting, plus three strict replays.
Aarch64, eight-CPU quota, 16 GiB, pinned nightly-2026-09-30 and locked dependencies.
Wall timer resolution is 0.01s; RSS is a whole-command memory proxy.

Median ordinary no-trace seconds:

| Workload | Debug before → after | Release before → after | Label-vector allocations removed |
| --- | --- | --- | ---: |
| minimal | 0.24 → 0.22 | 0.21 → 0.20 | 2 |
| read | 1.15 → 1.16 | 0.32 → 0.32 | 308 |
| inherent | 1.66 → 1.68 | 0.37 → 0.35 | 326 |
| shared-fanout | 0.88 → 0.89 | 0.31 → 0.31 | 252 |
| many-pairs | 1.69 → 1.64 | 0.37 → 0.33 | 4,608 |
| larger-graphs | 3.66 → 3.65 | 0.70 → 0.71 | 2,744 |

Counts are derived from successfully scored pair depths, `2 * max(h - 1, 0)`,
for these nonempty projected graphs. They describe label storage, not total
allocator activity. Trace has the same reduction because scoring and partitions
share refinement. Larger-graph trace medians were 4.11 → 4.13 seconds debug and
0.73 → 0.76 release. Samples are mixed and do not establish a general speedup;
the retained change removes repeated allocation without changing the algorithm.

All 120 complete matching reports compare exactly, including score detail bytes
and label partitions. Harness admission, exact-score, decision, completeness and
strict-replay assertions pass. All 681 nextest tests passed, zero skipped;
workspace doctests passed. Formatting, warning-free workspace/all-target Clippy,
24 study tests and two harness tests passed. Both required quality gates show
zero violations and zero actionable candidates; the same 15 reviewed ignores
remain. Logs: `/tmp/rstyle-wl-scratch-*`.
