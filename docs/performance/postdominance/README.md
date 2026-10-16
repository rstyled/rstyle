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
