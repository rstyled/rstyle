#!/bin/sh
# Regenerates site/wl/data/*.json (WL traces of the fixture pairs) and index.json.
# Usage: scripts/wl_site.sh [--serve]   (--serve starts a static server on :8765)
set -e
cd "$(dirname "$0")/.."
W=$PWD
env -u CARGO_TARGET_DIR cargo build -q -p rstyle-core --example wl_trace
env -u CARGO_TARGET_DIR cargo build -q --workspace
OUT=site/wl/data
mkdir -p "$OUT"

facts() { # project -> facts dir (runs the miner when the cache is missing)
  d=/tmp/rx_t_$1/rstyle/facts-sem
  if [ ! -d "$d" ]; then
    (cd "rust_refactoring_examples/$1" && CARGO_TARGET_DIR=/tmp/rx_t_$1 \
      RSTYLE_DRIVER=$W/target/debug/rstyle-driver \
      $W/target/debug/cargo-rstyle rstyle --format json patterns --all-targets >/dev/null 2>&1 || true)
  fi
  echo "$d"
}

# id | project | fn a | fn b | expect (high = should flag) | title | note
cat > /tmp/wl_pairs.txt <<'P'
b1|backup_sync|scanner::to_backup_key|sync::remote_key_for|high|B1 near clone (extra Path::components node)|Real target. One extra node costs 29 of 80 points.
q2|queue_worker|email_worker::retry_delay_seconds|image_worker::next_delay|high|Q2 small arithmetic fns|Real target. 5 of 6 labels match at level 0, score is 433.
clone|backup_sync|scanner::to_backup_key|scanner::to_backup_key|high|Clone (same fn twice)|Upper bound: every level matches.
t3|telemetry_workspace|file_agent::file_timestamp|http_agent::received_at|low|T3 decoy (timestamps)|Labelled keep-separate; 4-node graphs, not comparable.
t4|telemetry_workspace|file_agent::ingest_lines|http_agent::handle_request|low|T4 decoy (orchestration)|Same stages, different cardinality; should stay under the threshold.
P
echo '[' > "$OUT/index.json.tmp"
first=1
while IFS='|' read -r id project a b expect title note; do
  d=$(facts "$project")
  target/debug/examples/wl_trace "$a" "$b" "$d"/*.json > "$OUT/$id.json"
  [ $first = 1 ] || echo ',' >> "$OUT/index.json.tmp"
  first=0
  python3 -c 'import json,sys; print(json.dumps(dict(zip(["id","title","note","file","expect"],sys.argv[1:]))),end="")' \
    "$id" "$title" "$note" "$id.json" "$expect" >> "$OUT/index.json.tmp"
done < /tmp/wl_pairs.txt
echo ']' >> "$OUT/index.json.tmp"
mv "$OUT/index.json.tmp" "$OUT/index.json"
echo "wrote $OUT"
[ "$1" = "--serve" ] && cd site/wl && exec python3 -m http.server 8765
exit 0
