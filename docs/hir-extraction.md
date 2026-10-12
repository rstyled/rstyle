# Opt-in HIR extraction producer

Build with the repository's pinned toolchain, including rustc-dev:

```sh
cargo build --workspace --locked
RSTYLE_HIR_DOCUMENT_DIR=/tmp/rstyle-hir-documents \
  target/debug/rstyle-driver rustc --crate-type lib --edition 2021 \
  --emit metadata -C overflow-checks=yes -C debug-assertions=yes \
  -C panic=unwind -C opt-level=0 example.rs
```

The directory contains `facts.json` with captured source/HIR/build evidence,
per-function assessments, and accepted `function-N.pdg.json`,
`function-N.cfg.json` and `function-N.manifest.json`. Manifests identify documents
by their paired keys and SHA-256 of their exact serialized bytes. Unsupported
functions produce diagnostics and assessments without a graph pair. Use a fresh
output directory for each compilation so previous artifacts cannot be confused
with a failed run. `RSTYLE_HIR_FACTS_FILE` selects a separate facts path and can
also be used alone to capture frontend evidence.

Both wire documents follow the pinned extraction contracts in
`vendor-compat/rust-r0`. The native transport preserves record policies without conversion. For the narrow in-process
handoff, `producer::verify_serialized` validates manifests and captured semantic
evidence, then `rstyle_ccgraph::admit_produced` invokes strict admission. Raw
producer conformance flags are insufficient evidence.

Normal-flow validation and matching admission are separate from successful
compilation and from matching-policy gates. For example, a validated function
with fewer than the matching profile's required inputs/references can still be
excluded by those gates. The producer does not run matching or activate mining.

Applicable omitted panic/cleanup flow produces partial CFG evidence with explicit
limitations. Certification is bounded to the tested R0 subset; it does not claim
complete Rust execution, purity, equivalence or rewrite safety. Conditional body
source without complete source-resolution correspondence remains unknown.

See [the coverage ledger](../HIR_EXTRACTION_COVERAGE.md) for case-by-case results and
remaining coverage obligations. Validation commands:

```sh
cargo test -p rstyle-driver --test hir_extraction --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo rstyle --format json
cargo rstyle patterns --format json
```

Legacy extraction remains the default. Source-to-pair orchestration and detection
quality evaluation are outside this producer milestone.

The opt-in [Cargo source-to-pair workflow](source-to-pair.md) now connects this
producer to strict matching with isolated runs and verified serialized handoff.

## Versioned local coverage selections

`cargo rstyle clones --extraction-profile hir-calls-v1` explicitly selects the
largest current local bundle. The default remains pinned `r0`; smaller cumulative
bundles are `hir-const-v1`, `hir-casts-v1`, `hir-match-v1`, `hir-guards-v1`,
`hir-conditional-v1` and `hir-loops-v1`. Matching configuration is independent.
See [coverage contracts](extraction-profiles/coverage-v1.md) for complete syntax,
type, source-operation, CFG, dependence, projection and verification boundaries.

Const-body scope uses a local expansion because the pinned text does not settle
const function qualification. Casts remain explicit computation/dependency nodes;
match decisions and guard fallthrough retain source correspondence. Conditional
source inventory never resolves removed semantics from spelling. Cyclic flow uses
finite fixed points and divergence-sensitive postdominance without executable
virtual exits; expression-value merges in loop-containing functions are excluded.
The final profile admits only four primitive callee/type instantiations with
pinned compiler/core/source provenance, initially on aarch64-unknown-linux-gnu.

Every local paired document retains full captured frontend evidence and its
registered policy. Verification regenerates exact source inventories, normal-flow
operations and summaries; strict imported local admission repeats producer
verification. Conformance is local, not a change to the vendored R0 specification.
Unknown evidence and remaining unsupported owned source prevent emission of a
certified remainder. The [frozen inventory report](../eval/hir-coverage/REPORT.md)
records measured coverage, failures and remaining limitations separately from
historical study quality evidence.
