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
