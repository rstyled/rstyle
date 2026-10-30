#!/bin/sh
# Runs `cargo rstyle patterns --format json` over rust_refactoring_examples at several
# thresholds and prints which function sets get clustered. Usage: scripts/eval_examples.sh [thr...]
set -e
cd "$(dirname "$0")/.."
W=$PWD
cargo build -q --workspace
thresholds=${*:-"800 600 500 400 300"}
for d in telemetry_workspace backup_sync queue_worker feature_gate; do
  for t in $thresholds; do
    (cd rust_refactoring_examples/$d &&
      CARGO_TARGET_DIR=/tmp/rx_t_$d RSTYLE_DRIVER=$W/target/debug/rstyle-driver \
        $W/target/debug/cargo-rstyle rstyle --format json patterns --all-targets --threshold "$t" 2>/dev/null |
      python3 -c '
import json,sys
sys.path.insert(0,sys.argv[3])
from compiler_patterns import evaluation
r=evaluation(json.load(sys.stdin))
d,t=sys.argv[1:3]
cs=r.get("candidates",[]); sp=r.get("suppressed",[])
print(f"{d} thr={t} candidates={len(cs)} suppressed={len(sp)}")
for c in cs:
    print("   ", c.get("kind"), " | ".join(s["id"].split("::",1)[-1] for s in c["sites"]))
for c in sp:
    print("    suppressed:", " | ".join(s["id"].split("::",1)[-1] for s in c.get("sites",[])))
' "$d" "$t" "$W/scripts")
  done
done
