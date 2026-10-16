# Postdominance optimization evidence

## Contract inspected before editing

`producer/analysis.rs`, its `analyze_policy` consumers, `producer/lower/loops.rs`,
`producer/profile.rs`, and `docs/extraction-profiles/coverage-v1.md:193-215` define
normal flow. `rstyle-driver/tests/hir_extraction.rs` checks actual lowering and
exact dependence multisets, including the concrete while CFG at lines 2225ff:
`E -> n2 -> n3`, true `n3 -> n4 -> n5 -> n2`, false `n3 -> n6 -> n7 -> X`.
The source is `fn f(mut x: i32, b: bool) -> i32 { while b { x = 1; } x }`.
Only actual returning/fallthrough flow reaches X; infinite loops invent no exit.

Preserve all reachable-vertex postdominance pairs, including reflexivity and
cyclic mutual postdominance; preserve ordinary immediate postdominators;
preserve the exact ordered dependence vectors, outcomes, multiplicities and
boundary filtering. Cyclic control uses `post(y) minus strict-post(x)` for each
true/false transfer x->y, excluding E and X but retaining justified x->x.
Ordinary control follows immediate postdominators until ipost(x), suppressing X.
Execution still uses the unchanged greatest-fixed-point entry dominators;
reaching definitions, data edges, and ordinary invalid-input rejection are
outside these optimizations.

Both old algorithms are retained as test references. `baseline.txt` was captured
before changing production code (its two timed columns both execute the old
cyclic algorithm). `baseline-output.txt` records the old cyclic relation and
serialized control edges for the concrete lowered while topology; this later
capture uses the preserved old function. Synthetic transfers deliberately carry
both outcomes to test outcome multiplicity and distinct-successor counting.
The actual outcome labels and all dependence families are additionally checked
by the existing HIR integration tests.

## Commit 1: predecessor propagation

For candidate z, use the least fixed point of
`F_z(S) = {z} union {v | succ(v) nonempty and succ(v) subset S}`.
Indexed adjacency is constructed once from already distinct successor sets.
Scratch counters, admitted flags and queue capacity are reused across candidates.
For every **unadmitted** v, remaining[v] equals the number of distinct successors
whose queue events have **not been processed**. Admission and event processing
are different: an admitted successor can still have a pending event. Each
admitted vertex has exactly one event. z is admitted without any prerequisite;
all other vertices are admitted only by decrementing a positive counter to zero.
Original sinks therefore cannot be admitted unless they are z. Edges into an
already admitted vertex need no counter update, including z and self-loops.

Soundness follows by induction on admissions. Every nonseed admission has all
successors already admitted, so it belongs to the least fixed point.
At termination all admitted events have been processed, so every nonempty
successor set wholly inside the result has triggered admission. The result is
closed under F_z and contains z, proving equality with the least fixed point.
Admission ranks also prove the maximal-path interpretation: ranks strictly
decrease along every successor before z, forcing a visit to z. Outside the
fixed point a vertex is either an original sink other than z or has a successor
outside the fixed point. Repeatedly choosing that successor yields an original
sink or an infinite path (a remaining cycle in a finite graph). This establishes
the independent oracle: delete z, then look for an original non-z sink or a
remaining cycle, never a newly created sink. This covers avoiding cycles,
unavoidable deterministic cycles and mutual postdominance without virtual exits.

Propagation takes O(V(V+E)) worst-case time, including counter resets, instead of
potentially V rounds of whole-graph scans for each of V candidates. Indexed
relation storage costs O(R), adjacency O(V+E), scratch O(V). String set
materialization is measured separately and costs at most O(R log V) plus strings.
Dense relations can still require quadratic output storage; control output can
also dominate. The unchanged reachability, neighbourhood construction, entry
dominance and dataflow can remain expensive in the full analyzer.

Validation: every directed labelled graph on 0..4 vertices, including loops and
disconnected vertices (66,067 graphs), all candidates, agrees with the old
implementation and an independent DFS oracle detecting original sinks/repeated
vertices after excluding z. Exact complete `Dependence` control vectors are
compared against the old function on every graph using both true/false transfers.
Targeted E/X, chains, self-loops, unavoidable/avoiding cycles and the real lowered
while topology supplement enumeration. Existing package tests and all 47 HIR
extraction integration tests passed before editing and again before committing.

## Reproduction and measurements

Use the pinned nightly in `rust-toolchain.toml`. In this environment prepend
`/home/dev/.cargo/bin` to PATH. Commands from repository root:

```sh
cargo test -p rstyle-pdg
cargo test -p rstyle-driver --test hir_extraction
cargo test -p rstyle-pdg --release --lib benchmark_cyclic -- --ignored --nocapture
cargo test -p rstyle-pdg --lib captured_lowered_baseline -- --nocapture
cargo rstyle -p rstyle-pdg --all-targets --format json
cargo rstyle patterns -p rstyle-pdg --all-targets --format json
```

Raw repeated release timings are in `commit1.txt`. CSV columns:
`policy,family,V,E,R,control_edges,repeat,old_post_ns,new_post_ns,index_ns,
propagation_ns,representation_ns,control_ns,old_post_control_ns,
new_post_control_ns,old_vertex_scans,new_predecessor_visits`.
All durations are raw nanoseconds, three sequential repetitions, no subtraction
of timer overhead; indexing and materialization are included in `new_post_ns`.
`propagation_ns` includes scratch initialization/reset and indexed relation output.
Counters are computed outside timed regions in tests: old counts each vertex
predicate in every round including the final unchanged round; new counts every
predecessor-list visit, even skipped admitted predecessors. Relation pair count R
also gives queue events over all candidates. Counter units differ intentionally.
`family` deterministically sorts v0..vN lexicographically, then creates a chain,
a forward chain with two successors (`branch`), a deterministic ring (`cycle`),
or a chain with self-loops before its sink (`avoiding`). No randomness or external
input files. The tiny lowered fixture is a captured topology, not a measurement
of compiler/lowering time. Graphs are 32, 64, 128 vertices, plus that fixture.

Median microseconds, largest synthetic size (all raw repeats retained):

| Family | V/E | R / control edges | old post | new post | old post+control | new post+control | old scans / new visits |
|---|---:|---:|---:|---:|---:|---:|---:|
| chain | 128/127 | 8256/0 | 45874 | 365 | 48124 | 1368 | 1056768/8128 |
| branch | 128/253 | 255/502 | 1368 | 46 | 1477 | 186 | 32640/504 |
| cycle | 128/128 | 16384/256 | 144143 | 699 | 145162 | 2776 | 2097152/16384 |
| avoiding | 128/254 | 128/508 | 59 | 41 | 162 | 146 | 16384/254 |
| lowered while | 8/8 | 19/12 | 5.84 | 1.87 | 8.24 | 4.48 | 136/18 |

These are local microbenchmarks, not universal speedups. Cold allocation/cache
behavior and very small timings are noisy. The sparse avoiding case gains much
less because the old fixed point already terminates quickly. Baseline and new
are interleaved in the same release binary for matched comparisons.

Style scope: `cargo rstyle` and `cargo rstyle patterns` are run for all PDG targets.
Existing package-wide findings outside these files are not refactored as part of
this algorithm change. New/modified analysis functions must have no findings;
pattern candidates are inspected rather than suppressed.

## Commit 2: direct ordinary immediate postdominators

The ordinary consumer requires only membership `target in post(source)` and the
immediate-postdominator chain. It never enumerates postdominator sets. A direct
tree therefore avoids full-relation materialization. In particular, a chain has
R=V(V+1)/2 pairs but needs only V parents. The chosen representation stores
borrowed node names, parent/depth indices and binary-lifting jump rows. Only
tests reconstruct its relation; production never builds the ordinary Sets map.
This removes both convergence rounds and quadratic relation output, rather than
only making set evaluation faster. The common entry-dominator algorithm is
unchanged.

The existing `check_acyclic_exit` still runs first with the identical error string
and acceptance policy. It seeds X regardless of X's outgoing edges; that subtle
existing root convention is preserved. The new exit-order propagation seeds X
in the same way. All other vertices must have nonempty successors. When a
non-X vertex is admitted, every successor's event was processed already, so
every successor appears earlier in the queue. Queue length must equal V after
successful existing validation. Equivalently, non-X dependency edges form a DAG
when treating X as an absorbing root, even for malformed X-outgoing inputs.
Unreachable cycles still do not cause rejection.

Processing invariant: before attaching v, every successor's complete parent,
depth and jump rows are final and correct. The intersection of their ancestor
chains is the ancestor chain of their lowest common ancestor p. Every maximal
normal path from v begins with a successor, so p is exactly ipost(v); attach v
as a new leaf beneath p. This changes no ancestor chain of an existing vertex,
preserving the invariant. Initialize X as its own internal parent at depth zero;
its public immediate postdominator is None. Binary lifting equalizes depths,
then lifts both vertices from largest jump to smallest to find their LCA.
Membership lifts v to the candidate's depth and checks equality. These queries
reconstruct exactly the old reflexive relation, so the unchanged control-walk
logic emits identical ordered edges, outcomes and X boundary suppression.
Cyclic control continues using the first commit's full relation and self-control
rule; no tree is imposed on mutual postdominance.

Scheduling costs O(V+E). Tree construction costs O((V+E) log V), including
jump-row initialization, LCA folds and name indexing. Temporary adjacency costs
O(V+E); retained tree storage is O(V log V). There is no R-sized production
ordinary representation. Name lookup/membership and parent lookup cost O(log V),
so edge emission costs O((T+C) log V) plus output strings, for T inspected
transfers and C emitted control edges. The previous `immediate` repeatedly
scanned postdominator sets; the new parent query does not. The existing ordinary
validation, entry dominance, reaching definitions, and neighbourhood construction
remain unchanged and can dominate full-analyzer cost. These bounds describe the
replacement postdominator path, not the entire analyzer.

Exhaustive validation covers all 33,867 forward-edge graphs on 1..6 vertices in a
fixed topological labelling. All 10,106 graphs in which every non-X vertex reaches
X are checked against the old algorithm and an independent oracle enumerating
all paths to X and intersecting their vertex sets. The rejected DAGs are also
checked against the non-X-sink criterion. Every DAG has such a topological
labelling; this does not enumerate every permutation of labels. For every
accepted graph, tests reconstruct every pair from the compact tree, compare all
immediate postdominators, and compare full ordered control Dependence vectors
against the old emitter with both outcomes on every transfer. Targeted tests
cover E/X boundaries, diamonds, duplicate successor transfers, deep lifting
across multiple jump levels, the existing X-root convention, missing exits,
reachable self/other cycles and ignored unreachable cycles.

Before switching the ordinary production call, an ignored HIR integration test
captured actual lowered if/while graphs and **all** data/execution/control edges
into `crates/rstyle-pdg/tests/fixtures/postdominance_lowered.json`. The sources
are embedded in that file. A unit regression checks complete Analysis equality
with the first-commit algorithm and exact equality with these baseline edge
vectors; the capture test can independently regenerate the snapshots:

```sh
cargo test -p rstyle-driver --test hir_extraction capture_postdominance_baselines -- --ignored --nocapture
cargo test -p rstyle-pdg --release --lib benchmark_ -- --ignored --nocapture --test-threads=1
```

The second command produces `commit2.txt`, including a rerun of the cyclic
benchmark and the real lowered CFGs. Run benchmarks serially as shown, with no
other build/analysis work running. Old/new samples are interleaved; no warm-up
samples are silently removed. Environment: ARM aarch64 (Cortex-X925/A725 host),
nightly rustc 1.101.0-nightly (5c543b0b8, 2026-09-29), LLVM 23.1.1; ordinary
Cargo release optimization, no custom allocator or CPU pinning.

DAG CSV columns:
`dag,family,V,E,R,control_edges,repeat,old_post_ns,new_post_ns,index_ns,
schedule_ns,tree_allocation_ns,tree_propagation_ns,test_reconstruction_ns,control_ns,old_post_control_ns,
new_post_control_ns,old_vertex_evaluations,new_vertex_evaluations,
schedule_predecessor_visits,LCA_calls,jump_cells_written`.
The first-commit ordinary baseline is the unchanged old `dominators(...,"X")`
and old control emitter, not the pre-first-commit cyclic path. `new_post_ns`
includes indexing, scheduling and all tree representation construction. There
is no separate string-relation materialization in production: the explicitly
labelled reconstruction timing is test-only and excluded from new totals.
Tree representation allocation/initial fill is timed separately from LCA
propagation/attachment and scheduling. Jump-row writes during attachment are
intrinsic to the algorithm and included in propagation. Test-only operation counts derive exact calls/table writes
from the schedule and distinct outdegrees, with no counters in production.
Old evaluations include the final unchanged round; new includes X's initialization
and one attachment per other vertex. Predecessor visits count all original
adjacency entries, including ignored X-root entries in targeted cases.

`lowered-complete,cyclic,V,all_dependence_edges,repeat,first_commit_ns,new_ns`
measures the **whole analyzer**, including unchanged validation, entry dominance,
reachability, dataflow, and data/execution/control edge allocation. Its cyclic
baseline already uses commit 1; a cyclic difference here is noise, not a second
claim for the shared improvement. `lowered-original-cyclic,V,all_edges,repeat,
original_ns,first_commit_ns` supplies a supplemental complete-analyzer comparison
for commit 1 using the preserved original cyclic function. Synthetic control
benchmarks still deliberately use both outcomes on every transfer; lowered
complete-analyzer benchmarks use the actual captured transfers and operations.

Validation limitations: exhaustive bounds are finite and DAG label permutations
are not separately enumerated; large-graph confidence additionally rests on the
invariants above and targeted tests. Captured lowered benchmarks cover a scalar
if and while, not a broad real-world CFG corpus. Nanosecond-scale samples and
cold first samples are noisy. No claim is made that unrelated whole-analyzer
quadratic work has been removed.

Commit 2 medians in microseconds, against commit 1's ordinary path:

| Family | V/E | R / control edges | old post | new tree | old post+control | new post+control | old evaluations / new evaluations |
|---|---:|---:|---:|---:|---:|---:|---:|
| chain | 128/127 | 8256/0 | 161027.59 | 81.088 | 160775.09 | 178.000 | 16256/128 |
| branch | 128/253 | 255/502 | 91582.98 | 41.696 | 91345.64 | 185.504 | 8320/128 |
| lowered if | 7/7 | 21/4 | 20.58 | 0.896 | 21.68 | 2.160 | 42/7 |

For the 128-vertex chain/branch, scheduling visits 127/253 predecessor entries,
LCA folds make 0/126 calls, and attachments populate 1016 jump cells in either
case (excluding table allocation/initial fill). These counts are distinct from
membership queries and output walks; the latter are included in timings.

Complete actual lowered-if analysis (7 reachable vertices, 11 total dependence
edges) improved from median 124.59 to 89.46 us. The unchanged cyclic path (8
reachable vertices, 17 edges) measured 126.58 vs 117.42 us; this is noise and not an
improvement claim. The supplemental original-cyclic vs first-commit complete
analysis measured 122.56 vs 112.99 us in its separate three-sample group. These
small full-analyzer results show why the much larger postdominator-only gains
must not be presented as equivalent whole-pipeline speedups.

All raw samples are retained, including first samples prefixed by libtest's
`test ... ...` line when using one test thread. To parse, extract each CSV record
from its policy prefix (`cyclic,`, `dag,`, `lowered-complete,` or
`lowered-original-cyclic,`), rather than requiring the prefix at column zero.
Final validation results and style scope are recorded in `validation.txt`.

Stage timings are separate samples and need not sum to complete timings: cache
state and deallocation scopes differ. On sparse relations the jump table may
contain more indices than there are relation pairs; its benefit is a uniform
O(V log V) bound and avoiding repeated set scans, not a universal space reduction.
