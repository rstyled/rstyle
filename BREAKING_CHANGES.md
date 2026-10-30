# Compiler-only commands

`cargo rstyle`, `cargo rstyle patterns` and `cargo rstyle ignore` now use the
compiler capture pipeline exclusively. The pre-removal implementation is preserved
on branch [`legacy`](https://github.com/rstyled/rstyle/tree/legacy), at `753382ad`.

- Remove `--style-engine compiler` and `--engine compiler` from commands. Engine
  selectors, including their `legacy` values, are rejected before capture.
- The old flat `target/rstyle/facts` and `facts-sem` capture paths and the driver
  controls `RSTYLE_OUT_DIR`, `RSTYLE_ROOT` and `RSTYLE_PDG` are no longer producers.
  Use the saved run directory and its explicit unit snapshots for compiler replay.
  Old environment controls are still cleared when launching selected captures.
- Style JSON retains root diagnostics, summary, fix groups and complexity rows,
  together with compilation-unit labels and completeness. Compare deterministic
  findings and metrics, not entire reports containing fresh run paths.
  Failed compilation returns 2 with incomplete capture, never a clean summary.
- Pattern JSON uses `compiler_advisory.units[].patterns`: `evaluation` retains the
  raw proposals, while presentation and review indices select visible findings.
  Compilation contexts and unavailable inputs remain explicit. Strict matching is
  independent of advisory discovery.
- Existing `rstyle-allow` comments, `rstyle-ignore.toml` entries, saved compiler
  reviews and baselines continue through their existing workflows. Pruning needs
  complete workspace `--all-targets` input. No ledger conversion is required.

Shared Rust collectors, effect propagation, numeric evaluation, fix grouping,
parameter planning and the abstraction proposer remain in use. This removal does
not change strict clone admission, metric versions or thresholds and adds no hashes.
CI dog-food gates check compiler-default style and patterns.

Recorded reference/compiler comparisons stay under `eval/` with their original
scope, source revision and metric versions. Comparison readers operate on saved
JSON; a runnable reference engine is not a prerequisite. Graph research tools that
read retained flat facts are historical artifact readers, not production captures.
