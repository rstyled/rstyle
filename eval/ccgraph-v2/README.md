# Reproduce the bounded CCGraph follow-up

The immutable source protocol is preregistration commit `d626e7c`; see PROTOCOL.md
and the pre-execution counting correction in AMENDMENTS.md. Corpus signatures and
bodies are byte-identical excerpts. provenance.json links every natural excerpt
to the committed full upstream file and applicable license text. Do not treat
these purposefully selected excerpts as project-wide coverage or independent
human-reviewed labels.

Use the repository-pinned `nightly-2026-09-30` compiler. From repository root:

```sh
export PATH=/home/dev/.cargo/bin:$PATH
python3 -m unittest discover -s scripts -p 'test_ccgraph*.py' -v
cargo test --workspace --locked
cargo test -p cargo-rstyle --test cli --locked
cargo test -p rstyle-driver --test hir_extraction --locked
cargo test -p rstyle-ccgraph --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build -p cargo-rstyle -p rstyle-driver --locked
mkdir -p eval/ccgraph-v2/artifacts/frozen-bin
cp target/debug/cargo-rstyle target/debug/rstyle-driver eval/ccgraph-v2/artifacts/frozen-bin/
python3 scripts/ccgraph_followup.py validate
python3 scripts/ccgraph_followup.py smoke
python3 scripts/ccgraph_followup.py development
# Diagnose development evidence without changing source expectations/configuration.
# Commit results/development-binary-freeze.json before opening heldout.
python3 scripts/ccgraph_followup.py heldout
python3 scripts/ccgraph_followup.py summarize
```

Existing results are never overwritten. For reproduction, use a separate checkout
and move its committed results/ directory aside before executing; do not remove
the authoritative preserved run. Frozen binaries remain separate from Cargo's
build targets because orchestration regression tests rebuild sibling executables.
Each run invokes `cargo rstyle clones --format json --all-targets --locked
--offline --jobs 2 --max-pairs 1000 --trace --matching-config ... --manifest-path ...`.
No fixture matcher, AST path or legacy miner supplies predictions.

The v2 entry point reuses v1's validated command runner, mappings, exhaustive
pair verification, arithmetic and metrics. All study paths are explicitly passed;
only the command runner's process-local context is set to v2. The unchanged v1
script and pinned vendor tree are covered by the freeze. The v2 harness digest
and amendment digest are in every binary snapshot. Independent tests specify
coverage/oracle/target outcomes; the existing 15 bookkeeping tests exercise
mapping ambiguity, pair completeness, exclusions, incompatible pairs, resource
failure, drift and deterministic ordering. Synthetic fixtures never count as
study predictions.

Coverage primary targets and descriptive pair-quality secondary targets differ.
`summarize` exits 0 if all primary bounded targets are met, 1 for a valid miss or
insufficient primary evidence, 2 for invalid execution. Secondary quality
insufficiency remains explicit even when primary coverage targets are met.
`smoke`, `development` and `heldout` are execution-only checks, not quality claims.

Compact JSON per-scope outputs retain every actual pair, all stages and canonical
source pairs, including non-evaluable pairs and exclusions. summary.json uses
explicit population/split/configuration groups. Traces and evidence inventories
link committed results to raw local artifacts by SHA256. Raw compiler outputs,
Cargo targets and frozen binaries are ignored, reproducible, and never used as
an evidence cache. Source navigation is absolute file plus byte span, package,
target and compilation mode, not display name or digest alone. Repeated test
contexts are retained but normal-library contexts alone count toward quality.
