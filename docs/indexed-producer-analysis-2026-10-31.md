# Indexed producer graph analysis, 2026-10-31

The CFG/PDG producer previously scanned the transfer list repeatedly for reachability and neighbors, and rebuilt string-keyed dominance and reaching-definition maps during convergence. This change runs those calculations over one indexed topology and compact sets. Low implemented the change and executed the checks; Max independently reviewed the algorithms, semantic boundaries, and measurement evidence.

## Implementation

- `analysis/topology.rs` assigns deterministic local integer indices, builds deduplicated successor/predecessor adjacency, and finds reachable vertices with a queue. The reachable topology is reused by dominance, dataflow, exit admission, and postdominance. Unreachable vertices are removed before allocating iterative-analysis state.
- `analysis/bits.rs` supplies one small packed-bitset implementation for dominance and reaching definitions.
- `analysis/dominance.rs` intersects indexed predecessor sets and schedules only successors affected by a changed result. It computes the immediate dominators used for execution dependencies without materializing string-keyed dominance relations.
- `analysis/flow.rs` indexes the final formal-input seeds and first reachable operation generators. It evaluates only affected vertices, stages changed outputs, and commits each batch together. This preserves the original synchronous dataflow semantics, including entry-seed initialization. Queue membership storage is reused across batches. Definitions outside these seeds and generators do not enlarge the bitsets.
- `analysis/indexed.rs` indexes reachable operation occurrences once, retaining their original order and vertex indices for read, value, and execution edges. The first occurrence per vertex drives dataflow. Its definition-source table includes only active definition IDs and stops scanning once each has its first source. Original node/transfer order and duplicate occurrences are retained. The existing indexed postdominator tree and reverse-admission algorithm use the same topology.

The public `Graph` input, dependency identities, and `Analysis` result retain their existing representation for callers. String-keyed reaching-definition maps are materialized once at the result boundary. No alternative production algorithm or legacy-format support was added. The old scalar helpers live in a test-only reference module for independent algorithm comparisons.

Symbol and definition-source metadata still use borrowed string keys. A generated transfer looks up its symbol's indexed kill group; the converging per-vertex state uses packed sets.

## Correctness boundaries

The change preserves transfer-only vertices, the always-present entry `E`, the special exit-admission behavior of `X`, unreachable formal-input seeds, first-operation lookup for duplicate IDs, last-formal-input lookup for duplicate symbols, and first-global-source lookup for duplicate definition IDs. Source lookup includes unreachable operations and does not assume that a matching definition has the same symbol.

Any exception transfer still selects cyclic analysis, including an unreachable exception transfer. The distinction between acyclic and divergence-sensitive cyclic control dependencies, parameter/entry/exit suppression, error ordering, and emitted edge order and multiplicity is unchanged. Original proof checks and verification-policy boundaries remain intact.

## Measurement and validation

Baseline: commit `f7f09754`, frozen in a detached worktree before source edits. Scratch probes call the public `analyze` and `analyze_cyclic` APIs with the same deterministic inputs and matching locked dependencies and development-profile settings. Measurements use Linux ARM64 and the pinned `nightly-2026-09-30` toolchain; workspace code is unoptimized, as in the test builds. Evidence is retained under `target/perf-indexed-analysis-20261031/`.

Output comparisons cover complete ordered dependency edges, reachable vertices, input/output definition maps, and exact error strings. Timed loops include analysis and result destruction; process startup, input parsing, and output validation are outside the loop. Process CPU and maximum RSS include process startup and input parsing. Benchmark mode omits the separate output-serialization pass.

The 34 timing workloads cover two lowered fixtures, chains, branches, cycles, exceptions, and bindings at six sizes, plus two controls with many unreachable definitions. Each has three serial matched trials with alternating version order. The smallest graphs use 2,000 iterations per trial, lowered fixtures use 200, the unreachable-definition controls use 10,000, and the larger graphs use one. Separate repeated controls exposed setup overhead during development. Active-source pruning and reuse of reachable operation occurrences removed that regression before final measurements.

Final evidence is in `target/perf-indexed-analysis-20261031/candidate-v4/`. The earlier candidates are retained separately as development evidence. `measurement-summary.json` uses only the three final matched trials, excluding the separate preflight controls. All 34 workloads improved in this matrix; the smallest median improvement was 1.97×.

Selected results below use milliseconds per public analysis call. Ranges show the minimum and maximum of the three trials.

| Workload | Baseline median (range), ms | Indexed median (range), ms | Median speedup |
|---|---:|---:|---:|
| Lowered fixture 0 | 0.449 (0.446–0.451) | 0.0472 (0.0472–0.0921) | 9.51× |
| Lowered fixture 1 | 0.483 (0.482–0.544) | 0.0736 (0.0733–0.0736) | 6.56× |
| Chain, 192 operations | 3,177.86 (3,166.10–3,200.19) | 1.350 (1.331–2.506) | 2,353× |
| Bindings, 192 operations | 3,192.53 (3,187.34–3,197.20) | 2.406 (2.399–2.411) | 1,327× |
| Cycle, 192 operations | 3,164.94 (3,149.50–3,199.86) | 4.208 (2.435–4.240) | 752× |
| Exception, 192 operations | 3,160.72 (3,143.62–3,196.72) | 3.492 (1.973–3.518) | 905× |
| 128 unreachable definitions | 0.1040 (0.1039–0.1051) | 0.0427 (0.0425–0.0435) | 2.44× |
| 512 unreachable definitions | 0.2224 (0.2211–0.2252) | 0.1128 (0.1116–0.1134) | 1.97× |

Some indexed millisecond measurements vary by approximately 2×; the ranges are retained above. These results measure producer analysis in debug builds, not a whole build or CI run.

Median maximum process RSS fell from 7,592 to 2,792 KiB for the chain and from 7,592 to 3,272 KiB for bindings. The smaller fixtures and unreachable-definition controls remain near the process RSS floor; the 512-definition control reports 2,952 KiB for both versions. RSS includes the parsed graph and runtime and is not an allocation count or process-tree memory profile. For 10,000 calls of that control, median user CPU fell from 2.22 to 1.13 seconds. GNU time's 0.01-second resolution rounds many indexed single-call CPU measurements to zero.

Validation:

- Complete ordered output, exact errors, and standalone reachability match the baseline on 1,602 correctness cases and all 34 timing inputs. These include exhaustive small directed graphs, seeded multi-symbol graphs across bitset-word boundaries, duplicate IDs, and entry/exit special cases.
- Focused producer tests pass: 11 passed, zero failed. Four existing ignored benchmark tests retain their original ignore status.
- The actual rebuilt CLI/driver pass the numeric and pattern gates and the saved-pattern-status reader. Numeric analysis measures all 10,181 functions with zero violations and zero unavailable complexity measurements. The fresh capture includes all 65 production analysis observations, including the new helpers. Pattern input is complete, with zero unignored candidates. Existing thresholds, suppressions, and review ignores are unchanged.
- `cargo nextest run --workspace --locked --profile ci`: 3,789 passed, zero failed, five existing tests skipped; 237.878 seconds. A fresh JUnit report is retained at `workspace-checks/junit.xml` in the final artifact directory.
- Workspace doctests, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo fmt --all -- --check` pass.
- Source hashes match the measurement manifest, and source/verified CLI/driver hashes remain unchanged across all final checks. `checks-complete.json` records the successful freeze checks.

The local artifact directory retains `final-manifest.json`, `checked-source-hashes.json`, `checked-binary-hashes.json`, `fidelity-validation.stdout.log`, `measurement-summary.json`, per-trial command/result/CPU/RSS files, and actual gate reports under `style-verified/`.

## Scope

This optimizes producer graph analysis. It does not change extraction coverage, matching policy, exhaustive function-pair enumeration, report completeness, CI triggers, style limits, suppressions, or reviewed pattern ignores.

Reachability’s queue traversal visits each reachable vertex and indexed adjacency entry once. Building the deterministic index and sorting/deduplicating adjacency also have costs; the complete analysis is not claimed to be linear. Dominance uses packed per-vertex sets, and reaching-definition memory depends on the number of relevant symbol/definition pairs. Cyclic postdominance and large public result maps can still require quadratic space or work. Whole-CI and other-platform gains require separate measurements.
