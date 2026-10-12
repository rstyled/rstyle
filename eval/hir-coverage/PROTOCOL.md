# Frozen eligibility inventory v1

Frozen before new extraction or detector observations on this inventory.
This is an eligibility inventory, not a quality study. No labels, calibration,
accepted-pair quality conclusions, or unseen-holdout claims are permitted.

Packages are crates.io lz4_flex 0.14.0, regex-syntax 0.8.11 and time-core 0.1.9.
Archive SHA-256 identities, respectively:

- ecbdfe44b1bd960b68170b417450a628c43f7cf56bb3c5317e61cb230ee7f226
- d6f6ff9a378485b298a5286656da665ba74413d36db0979633275d2e708145d4
- 9e1c906769ad99c88eaa54e728060edef082f8e358ff32030cb7c7d315e81109

Select each package's library target, default features, debug settings, host
x86_64-unknown-linux-gnu, nightly-2026-09-30. Dependencies are compiled but not
sampled. Enumerate source function definitions (including methods, unsupported
signatures and conditionally absent definitions) in all src/**/*.rs files using
tree-sitter-rust 0.24.0, tree-sitter 0.25.2. Sort by relative path and start byte;
select the first 24 definitions per package, or all if fewer exist. Do not filter
by scalar types, compilation, eligibility, detector output or presumed utility.
Retain every selected definition's exact bytes, path, span and digest before
running capture. Macro token bodies are not parsed definitions; disclose this
boundary. This bounded selection is not whole-crate Rust coverage.

Compile the unmodified complete packages with the actual Cargo capture workflow.
A selected definition absent from resolved HIR is unknown with conditional or
capture evidence, never an invented resolution. Failed package compilation is a
failed outcome for every affected selection. Inventory runs use --max-pairs 0:
no detector prediction is inspected and pair-budget incompleteness is separate
from capture/assessment completeness. Retain all eligibility diagnostics and an
independent source blocker traversal, including nested unsupported constructs.

For each slice, compare the same selected source identities. Record admitted,
excluded, unknown and failed, all blocker combinations, functions blocked only
by a proposed capability and potentially unlocked bundles. Potential unlocks
are hypothetical until actual verified admission; diagnostic counts are not
coverage gains. Freeze call-summary selection only after source review of this
inventory. Preserve ccgraph-v1 and ccgraph-v2 without relabeling their evidence.

## Development output policy

Frozen inputs, provenance, dependency lockfiles and independent expectations are
committed. Full captures, stage diagnostics and comparisons are regenerated with
`uv run eval/hir-coverage/reproduce.py` into ignored `target/hir-coverage/`.
Do not commit raw run outputs or compare transient paths/log bytes as fixtures.
Keep the compact report tied to its tested implementation revision. The corrected
host and development failures are disclosed in AMENDMENTS.md. Runtime evidence
verification remains strict; the two historical studies remain unchanged.

Current development compares R0 with the complete `hir-expanded-v1` bundle.
Intermediate stages are historical contracts, not production Cargo choices;
README.md gives the immutable checkout for reproducing their original comparison.
