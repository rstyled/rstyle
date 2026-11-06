# Runtime complexity: next work

The implementation branch is `feat/patterns-runtime-bounds`; the validated foundation at handoff is commit `cf1fa8b0`. This plan describes the next analysis phases.

Repository: https://github.com/rstyled/rstyle. Research background: [Bound Analysis of Imperative Programs with the Size-change Abstraction](https://www.microsoft.com/en-us/research/publication/bound-analysis-imperative-programs-size-change-abstraction/). The current feature implements a scoped analysis, not the full research system.

## Current implementation and evidence

`patterns --complexity` opts into compiler HIR capture, lowers typed bodies to a dense semantic CFG, and analyzes that owned input in a pure Rust engine. Observations travel in existing frontend options and snapshots. Capture capability checks prevent replay from silently using facts without runtime inputs; saved analysis does not need the compiler. Ordinary pattern defaults, proposal identities, and gates remain separate.

Current results bound local loop-body entries and modeled call-site invocations per owner invocation. They do not compose callee work, byte counts, allocation costs, or whole runtime. Vetted collection length and traversal adapters are excluded from modeled call counts. Private unsigned norms and stable standard extents have conservative alias and control checks; unsupported behavior remains `Unknown`, and resource exhaustion remains distinct from missing observations.

The fresh self-search inspected **10,368 context-owner rows** across six units: **4,468 Proven**, **5,900 Unknown**, no missing inputs or budget results. Of the proven rows, **115 functions contain 116 certified loop sites**: 114 functions have degree-one bounds and one has constant bound 2. The remaining 4,353 proven rows have no modeled loops. All whole-runtime results are unknown. Equal polynomial degree does not rank elapsed time; `Unknown` does not mean fast. The current search establishes no quadratic runtime bound on our expensive matching algorithms.

Local, ignored evidence:

- `target/runtime-bounds-self-search-20261102/`: fresh command, report, coverage summary, bounded rows, and source-checked review findings.
- `target/runtime-bounds-implementation-v2-20261102/`: implementation validation, frozen binaries, regression runs, and numeric/pattern gates.

These artifacts may be absent in another checkout; source and tests are authoritative.

Reproduce the baseline from the repository root with the pinned toolchain:

```sh
cargo build -p cargo-rstyle -p rstyle-driver --bins
rstyle_bin="${CARGO_TARGET_DIR:-target}/debug/cargo-rstyle"
"$rstyle_bin" rstyle patterns --complexity --top 30 --format json
```

The report's `run_directory` identifies the saved capture. Pass it to the same binary with `rstyle patterns --complexity --replay-run RUN --format json` for replay. Use the prepared binary directly for measurements so compilation time is recorded separately.

## Phase 1: standard iterators and closure multiplicity

Start with `medoid` in `crates/rstyle-core/src/cluster.rs`. Its outer `min_by_key` callback invokes `total`, which filters and maps the group before summing similarity scores. Model resolved standard operations, inline adapter closures, and statically resolved immutable local closures such as `total`. Preserve capture identities, evaluation order, and invocation multiplicities across these layers. A callback is not one call per function. Establish a sound quadratic upper bound on `similarity_milli` invocation counts; an initial conservative `g²` bound is sufficient, while a tighter `g*(g-1)` claim requires justified distinctness and filter reasoning. This still does not bound the cost of each similarity calculation.

Then cover focused pair-generation chains using `.enumerate()`, `.skip()`, and `.flat_map()`. Track independent cardinalities, callback captures, short-circuiting, and adapter-specific invocation limits. Resolve library identities through compiler facts; do not infer contracts from method spelling, arbitrary `Iterator`, or `size_hint`. Keep unresolved callbacks, unsafe effects, mutable captures affecting norms/extents, and unsupported guarded behavior unknown. Bound capture and analysis work.

Acceptance: meaningful pure Rust tests for callback/cardinality composition and negative cases; compiler regressions added to the existing batched fixture; a real `medoid` result showing the expected quadratic call-count bound with its assumptions and source evidence. Preserve existing reset, persistent-cursor, continue, alias, pattern-effect, and replay regressions.

## Phase 2: compose callee summaries

Within the existing HIR compiler invocation, capture stable callee identity plus call arguments and their size-measure mapping. Current `Action::Call` only stores a display name and an alias flag, which is insufficient for reliable composition. HIR is sufficient for these scoped phases; no new compiler pass or MIR pipeline is needed.

Build summaries bottom-up over resolved calls. Keep recursion, unresolved dispatch, unavailable bodies, and alias effects conservatively unknown until explicitly modeled. Preserve multiple independent sizes instead of replacing them with one generic `n`.

Start by composing loop-entry and call-invocation counts. Define the cost unit and primitive/library cost summaries before promoting those counts to operation-cost bounds.

Per-item varying callee sizes require symbolic sums or a justified maximum-size measure. The current `Bound` AST has only `Const`, `Input`, `Add`, `Mul`, and `Max`; it cannot express indexed sums. Design that representation and its assumptions before claiming a composed total. Iterator `.map()` support alone does not establish BFS or worklist complexity: queue growth, visited-set invariants, and repeated traversal need separate reasoning.

Acceptance: summary substitution tests with independent and varying sizes, conservative recursive/unresolved cases, saved replay without compiler access, and source-checked results on real callers. Keep local-count, composed-cost, and unknown scopes explicit.

## Phase 3: measure actual hotspots

Profiling can run independently now. Profile the self-search and representative real inputs to find actual expensive paths. Useful review targets include repeated preparation/admission, graph projection, verification/reconstruction, and serialization. Compare matched inputs, options, and compiled binary hashes; retain raw evidence and distinguish startup/build work from analysis.

Validate static claims against real functions and measured call counts where practical. Use timing and allocation evidence to prioritize optimization. Do not turn equal degree-one bounds or unknown rows into a wall-clock ranking.

## Repository map

- Lowering from documents: `crates/rstyle-core/src/cpg/bounds/`.
- Pure engine: `crates/rstyle-core/src/runtime_bounds/{ir,symbolic,analysis,tests}.rs` and `analysis/{loops,values}.rs`.
- Observation schema and capture limits: `crates/rstyle-core/src/advisory/runtime_bounds.rs`.
- CLI results and routing: `crates/cargo-rstyle/src/compiler/advisory/runtime_bounds.rs` and `crates/cargo-rstyle/src/compiler/patterns_route.rs`.
- Batched compiler integration fixture: `crates/cargo-rstyle/tests/compiler_advisory/runtime_bounds.rs`.
- Pattern clustering and similarity: `crates/rstyle-core/src/cluster.rs` and `wl.rs`.

## Execution and review constraints

Future implementation should use Low for implementation and independent Max review, including soundness counterexamples before adoption. This is not a production compatibility surface: breaking changes are allowed; do not add legacy compatibility scaffolding. Do not change CI, dependencies, build profiles, thresholds, suppressions, or ignore ledgers to make checks pass.

Use focused pure Rust unit tests and reuse the batched compiler fixture before full validation. Run formatting, appropriate Clippy/tests, and both real numeric and saved pattern gates through `scripts/ci-style.py` with freshly built binaries. Preserve failed and successful evidence separately.

`rust-toolchain.toml` pins `nightly-2026-09-30` and the rustc development components. Ensure `/home/dev/.cargo/bin` is on `PATH`. The existing normal local build cache is `/home/dev/src/rstyle-main/rstyle/target/perf-investigation/live-20261031/build-target`; it is an optional local convenience, not a repository requirement. Typical future checks are `cargo test -p rstyle-core runtime_bounds` and `cargo test -p cargo-rstyle --test cli compiler_advisory::runtime_bounds`. For the style workflow use `python3 scripts/ci-style.py --binary <fresh-cargo-rstyle> --output <new-directory>`, then its separate `--patterns-status` mode. Coordinate heavy jobs and use new artifact directories.
