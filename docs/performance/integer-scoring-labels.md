# Integer histogram scoring

The strict matching pipeline now scores the existing borrowed `u64` WL
histograms directly. It no longer creates a second vector of string-keyed
histogram trees for all rounds before computing overlap, cross kernels and
self kernels. The public string-histogram scoring API remains a narrow adapter
to the same generic exact arithmetic; there is one implementation of scoring.

`L…` strings are presentation labels, created only when rendering stage details.
The presentation adapter sorts those labels lexically and uses the same debug
map encoding, preserving existing `round_N_a` / `round_N_b` detail bytes, including
the order of `L10` and `L2`. It builds one temporary vector for a histogram at a
time instead of retaining all string-keyed trees alongside all numeric trees.
Numeric labels remain pair-local and never become graph-global cache keys.

The generic arithmetic has the same checked additions and multiplications,
weights, kernels, fractions and error conditions. It compares full numeric
labels produced by the shared pair interner. No thresholds, graph semantics,
pair enumeration, completeness or admission policy change. All existing stage
detail fields remain present in the scoring API and trace reports. This does
not yet eliminate detail generation for non-trace CLI output or share trace
refinement with score refinement; those remain separate work.

Regressions compare integer and readable histogram scoring exactly, including
labels whose lexical order differs from numeric order. They retain overflow and
round/weight mismatch errors. Presentation coverage checks exact old map-debug
bytes for empty histograms and labels up to `u64::MAX`. Existing exact-boundary,
renaming/reordering, self-edge, duplicate-edge and scoring fixtures remain enabled.

The structural allocation improvement is removal of `2 * (depth + 1)` copied
histogram trees per evaluated WL pair. Label strings remain necessary for the
existing detail format, but only one histogram's temporary names remain live at
a time. The existing numeric histogram trees and final detail strings remain.

## Measurements and validation

Baseline source `f8f494d` is the retained previous candidate, with saved binaries
in `/tmp/rstyle-before-integer-scoring/{debug,release}`. Candidate is this commit.
[312 raw command samples](integer-scoring-samples.json) include actual embedded
producer revisions, peak RSS and structural counts. The same source paths and
warm fixtures are used for both revisions. Aarch64, eight-CPU quota, 16 GiB,
nightly-2026-09-30, locked dependencies; prebuilt debug/release analyzer/driver
binaries analyze the same ordinary check-profile fixtures. Build/setup is
excluded and no tests or builds overlap timings. One warmup, three ordinary and
one separate diagnostic sample per trace setting; three strict imports per case
and analyzer profile. Wall timer resolution is 0.01 seconds.

The first candidate broad run overlapped an extra report comparison. Its timings
were discarded; the entire broad candidate run was repeated in isolation. Only
that isolated run appears in the retained sample data. Complete-report comparisons
were performed after measurement completion.

Median ordinary no-trace wall seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.24 → 0.23 | 0.20 → 0.20 |
| read | 1.12 → 1.12 | 0.32 → 0.30 |
| inherent | 1.63 → 1.67 | 0.35 → 0.38 |
| shared-fanout | 0.89 → 0.89 | 0.29 → 0.29 |
| many-pairs | 1.65 → 1.65 | 0.35 → 0.37 |
| larger-graphs | 3.68 → 3.64 | 0.73 → 0.72 |

These differences do not establish a uniform command or suite-wide speedup.
The structural reduction, per complete command and independent of trace mode:

| Workload | Copied trees removed | Copied tree entries removed |
| --- | --- | --- |
| minimal | 6 | 14 |
| read | 632 | 2691 |
| inherent | 650 | 2743 |
| shared-fanout | 336 | 6510 |
| many-pairs | 9312 | 32496 |
| larger-graphs | 2856 | 147840 |

These counts describe the removed representation, not allocator event counts.
The final detail strings and temporary presentation-label strings remain.
Ordinary peak-RSS samples cover the whole command, including compiler work;
they are not an attribution of histogram storage alone.

All 120 complete matching reports compare equal between baseline and candidate,
including histogram detail bytes and trace partitions. The measurement harness
also checks exact score inputs, stage outcomes, admission, exhaustive enumeration
and completeness. All strict imports passed.

```sh
python3 scripts/measure_library_resolution.py after target /tmp/rstyle-fast-checksum-measurement
timeout 900 python3 scripts/measure_broad_pipeline.py after target /tmp/rstyle-fast-checksum-broad
```

675 nextest tests passed, zero skipped; workspace doctests passed; warning-free
workspace/all-target Clippy, formatting, focused ccgraph tests, 24 study
bookkeeping and two measurement-harness Python tests passed. Both required gates
report zero style violations and zero actionable pattern candidates; 15 existing
reviewed ignores are unchanged. Logs: `/tmp/rstyle-integer-scoring-*`.
