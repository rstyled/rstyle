# Bounded strict CCGraph study v1

See PROTOCOL.md, freeze.json and manifest.json for preregistered inputs, source-only
labels and splits; AMENDMENTS.md records infrastructure failures/retries.
This study is separate from the legacy patterns evaluation directories.

From the repository root with Python 3 (standard library only), GNU time and the
pinned nightly-2026-09-30 compiler/rustc-dev installed:

```sh
export PATH="/home/dev/.cargo/bin:$PATH" # adapt only this installation location
cargo build -p cargo-rstyle -p rstyle-driver --locked
mkdir -p eval/ccgraph-v1/artifacts/frozen-bin
cp target/debug/cargo-rstyle target/debug/rstyle-driver eval/ccgraph-v1/artifacts/frozen-bin/
python3 scripts/ccgraph_study.py validate
python3 -m unittest discover -s scripts -p test_ccgraph_study.py -v
python3 scripts/ccgraph_study.py smoke
python3 scripts/ccgraph_study.py development
# Inspect development only; keep configs/labels/splits unchanged.
python3 scripts/ccgraph_study.py heldout
python3 scripts/ccgraph_study.py summarize # exit 1 means valid insufficient/missed targets
python3 scripts/ccgraph_study_evidence.py
```

Committed evaluations are immutable: these commands refuse to overwrite successful
results. For an explicitly disclosed reproduction, use a separate checkout, preserve
committed originals elsewhere, and move its results directory aside before running.
Do not call a reproduction an unseen holdout. Freeze/config validation uses paths
relative to the repository and source mapping uses navigation plus compilation
context. Edit the harness's rustup installation path if necessary; record that as
an infrastructure adaptation. No semantic input changes are needed. The command
creates fresh compilation/capture directories each time and never scans old runs.
Do not run a build that replaces study binaries during execution: the copied
executables isolate the run from Cargo test's sibling-driver build. Compilation
and matching cost is combined because the existing command has no phase timers.

`validate`, `smoke`, `development`, `heldout` return 0 for completed execution only,
2 for invalid/incomplete execution. `summarize` is the quality target check: 0 for
all primary heldout targets met, 1 for valid misses/insufficient evidence, 2 for
invalid/incomplete inputs/results. Target miss is not compiler failure.

Results include every emitted context pair and its seven stages, all canonical
source-pair outcomes (including exclusions), function mappings/reasons/projections,
per-stratum/per-group metrics and independent-group counts. Per-group negative
pairs can belong to both groups: these descriptive overlapping summaries are not
independent replicates and must not be summed. Populations remain separate. No
confidence intervals assuming independent pairs are provided for this small
correlated convenience sample. Genuine human label review remains future work.

Full raw reports, captured HIR, serialized graphs/manifests/ledgers, and fresh
build output remain under ignored artifacts/ and corpus/**/target/. Committed
semantic results identify exact raw-report SHA256 and evidence-inventory identity;
evidence-index.json inventories retained run evidence. These byte identities are
machine/run-specific. Recompilation regenerates semantically comparable outputs,
not identical absolute paths or report bytes. Frozen source/config/contract hashes
are portable; executable hashes and compiler/Cargo versions identify the actual
run. Build debug paths, producer metadata and compiler arguments are retained in
raw reports; committed identities distinguish producer build metadata from the
unchanged baseline production tree. No arbitrary target directories are committed.

Complete reproduction validation commands:

```sh
cargo test -p cargo-rstyle --test cli --locked
cargo test -p rstyle-driver --test hir_extraction --locked
cargo test -p rstyle-ccgraph --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
PATH="$PWD/target/debug:$PATH" cargo rstyle --format json
PATH="$PWD/target/debug:$PATH" cargo rstyle patterns --format json --fail-on-candidates
```

The last command checks existing repository legacy pattern hygiene as requested;
no legacy miner supplies study predictions, and no grouping/proposal/rewrite is
activated. No matching/extraction/default settings are changed.
