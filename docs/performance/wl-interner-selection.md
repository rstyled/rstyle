# Complete-key WL interner selection

Strict WL already used `BTreeMap<WlTuple, u64>`, not SHA-256. The separate
`rstyle-core` FNV implementation is unchanged. This increment benchmarks the
existing get/insert tree, an entry-based tree, FNV-1a hash-map entries and XXH3
hash-map entries. Every backend retains the complete tuple and checks equality;
none substitutes hash-only colors for the refinement relation.

The entry-based tree is retained in production. It avoids a second traversal
when a new tuple is inserted. Labels still allocate from the table length in
the exact graph/node visitation order. No label or report ordering comes from
container iteration. Both graphs share one vocabulary, including all synchronous
rounds; initial categories, incoming/outgoing direction, sorted family/label
multisets, duplicate edges, self-edges, depth, weights and scoring are unchanged.
No lookup-hashing dependency or alternative runtime is added to the production
matcher. The existing XXH3 dependency is reused only as a development dependency
for the interner benchmark.

`wl_interner_bench` compiles the same complete tuple type as production. It visits
128/1024/4096 distinct keys three times, with 1/2/4/32/256 neighbors per direction.
Each case tests both early distinctions and a shared label/neighbor prefix with
the last neighbor differing. This covers cases favorable to trees and cases
requiring long comparisons. All variants assert the same exact label sequence
and unique count against the incumbent tree, not merely equal hash values.
Each variant receives owned tuples (including two cloned neighbor vectors per
visit), matching production's owned tuple insertion interface. Construction of
the benchmark input keys is outside the measured operation.

One warmup precedes three raw samples per variant/case/profile. These isolated
benchmarks are separate from ordinary pipeline timing. Hash-map alternatives
were slower than the entry tree in the measured release cases. FNV wins two
debug shared-prefix cases, but loses their release equivalents. This evidence
does not justify switching the normal matcher to either hash map.

An intentionally constant test hasher forces all keys into collisions. Distinct
previous labels, neighbor labels, families, multiplicity and direction remain
distinct; repeated complete tuples reuse IDs. This exercises the evaluated hash
lookup model. The retained production tree uses complete ordering/equality and
does not depend on hash collision behavior.

Tuple/vector allocations and retained interner key counts are unchanged by the
production entry change. In the benchmark, the common construction proxy is two
neighbor-vector allocations per visit and `2 * neighbors * sizeof((u8,u64))`
payload bytes per visit, independent of backend. Table layouts differ; benchmark
process peak RSS is recorded as an aggregate, not an allocator-event count or
per-backend memory attribution. Matched production RSS samples compare the actual
retained tree change against its incumbent. Scratch reuse and redundant trace
refinement remain separate measured work rather than claimed improvements here.

## Measurements and validation

Baseline source: `e9e82a3`; retained checksum-candidate samples and exact binaries
are reused without repeating baseline commands. Saved binaries are in
`/tmp/rstyle-before-wl-interning/{debug,release}`. Candidate is this commit.
The [312 pipeline samples and interner samples](wl-interner-samples.json) record
embedded producer revisions. Eight-CPU aarch64 host, 16 GiB, pinned nightly,
locked dependencies, debug/release analyzer profiles, ordinary check fixtures,
matching source paths and warm state. Builds are excluded and never overlap
tests or measurements. One warmup, three ordinary samples and one separate
diagnostic run per trace setting; three strict imports per workload/profile.

Median ordinary no-trace command wall seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.23 → 0.24 | 0.21 → 0.20 |
| read | 1.17 → 1.12 | 0.32 → 0.32 |
| inherent | 1.70 → 1.63 | 0.34 → 0.35 |
| shared-fanout | 0.88 → 0.89 | 0.30 → 0.29 |
| many-pairs | 1.66 → 1.65 | 0.39 → 0.35 |
| larger-graphs | 3.72 → 3.68 | 0.70 → 0.73 |

All decisions, exact score inputs, stage outcomes, admission, pair counts and
completeness passed parity in both profiles and trace modes. All strict-import
replays succeeded. Command timings are mixed and do not show a universal gain;
no suite-wide speedup is inferred. The 0.01-second timer and per-command peak RSS
samples are retained. The production container and owned-key allocations remain
the same, so this change claims fewer traversals rather than reduced key memory.

Isolated median lookup milliseconds; all cases use three visits per key:

| Profile | Keys / neighbors / shared prefix | Incumbent tree | Entry tree | FNV map | XXH3 map |
| --- | --- | --- | --- | --- | --- |
| debug | 128 / 1 / False | 0.456 | 0.387 | 0.424 | 0.570 |
| debug | 128 / 1 / True | 0.304 | 0.253 | 0.257 | 0.320 |
| debug | 1024 / 2 / False | 2.900 | 2.432 | 2.610 | 3.151 |
| debug | 1024 / 2 / True | 3.889 | 3.078 | 2.609 | 3.134 |
| debug | 4096 / 4 / False | 14.752 | 11.942 | 14.551 | 17.647 |
| debug | 4096 / 4 / True | 23.735 | 18.434 | 14.531 | 17.502 |
| debug | 1024 / 32 / False | 3.902 | 3.419 | 18.112 | 24.785 |
| debug | 1024 / 32 / True | 17.026 | 12.970 | 18.176 | 24.903 |
| debug | 128 / 256 / False | 1.281 | 1.248 | 16.799 | 22.636 |
| debug | 128 / 256 / True | 8.824 | 6.899 | 16.860 | 22.591 |
| release | 128 / 1 / False | 0.019 | 0.018 | 0.029 | 0.028 |
| release | 128 / 1 / True | 0.019 | 0.018 | 0.028 | 0.028 |
| release | 1024 / 2 / False | 0.195 | 0.191 | 0.324 | 0.322 |
| release | 1024 / 2 / True | 0.258 | 0.226 | 0.325 | 0.315 |
| release | 4096 / 4 / False | 1.065 | 0.943 | 1.762 | 1.587 |
| release | 4096 / 4 / True | 1.602 | 1.314 | 1.780 | 1.607 |
| release | 1024 / 32 / False | 0.341 | 0.340 | 2.429 | 1.813 |
| release | 1024 / 32 / True | 1.416 | 1.164 | 2.427 | 1.812 |
| release | 128 / 256 / False | 0.159 | 0.160 | 2.217 | 1.624 |
| release | 128 / 256 / True | 0.880 | 0.713 | 2.233 | 1.664 |

Aggregate microbenchmark peak RSS: debug 4872 KiB; release 4680 KiB.
This process includes all candidate backends and does not isolate their memory.
The retained production change is compared using ordinary command RSS samples;
no claim is made that a hash map has lower memory use.

```sh
cargo build --locked -p rstyle-ccgraph --example wl_interner_bench
cargo build --release --locked -p rstyle-ccgraph --example wl_interner_bench
/usr/bin/time -v target/debug/examples/wl_interner_bench
/usr/bin/time -v target/release/examples/wl_interner_bench
python3 scripts/measure_library_resolution.py after target /tmp/rstyle-fast-checksum-measurement
python3 scripts/measure_broad_pipeline.py after target /tmp/rstyle-fast-checksum-broad
```

672 nextest tests passed, zero skipped; workspace doctests passed; warning-free
workspace/all-target Clippy and formatting passed; the full ccgraph focused
suite, 24 study bookkeeping and two measurement-harness Python tests passed.
Both quality gates are clean: zero violations and zero actionable candidates,
with 15 existing reviewed ignores unchanged. Logs: `/tmp/rstyle-wl-interner-*`.

This does not complete the broader optimization task. Source wire dictionaries,
remaining manifest/declaration boundary reuse, integer scoring labels, scratch
allocation and shared trace/scoring refinement remain to be implemented and
measured before the requirement-by-requirement completion audit.
