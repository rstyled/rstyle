#!/bin/sh
# Regenerates eval/align/pairs.jsonl (gitignored, ~16 MB) and the summary CSVs.
# Needs facts caches from scripts/loo_eval.py runs: /tmp/rx_t_<project>/rstyle/facts-sem.
set -e
cd "$(dirname "$0")/.."
env -u CARGO_TARGET_DIR cargo build --release -q -p rstyle-core --example align_ablation
mkdir -p eval/align
: > eval/align/pairs.jsonl
for p in telemetry_workspace backup_sync queue_worker feature_gate tarpc mundane argh; do
  target/release/examples/align_ablation "$p" /tmp/rx_t_$p/rstyle/facts-sem/*.json >> eval/align/pairs.jsonl
done
python3 scripts/align_ablation.py
