#!/bin/sh
# Raw vs canonical WL / alignment coverage for the labeled pairs.
# Usage: scripts/norm_pairs.sh <rules>     (needs a prior `scripts/loo_eval.py` or `wl_site.sh` run for the facts)
set -e
cd "$(dirname "$0")/.."
env -u CARGO_TARGET_DIR cargo build -q -p rstyle-core --example norm_pairs
BIN=target/debug/examples/norm_pairs
F() { ls /tmp/rx_t_$1/rstyle/facts-sem/*.json; }
run() { # project, pairs on stdin
  p=$1; shift
  $BIN "$1" $(F $p) | tail -n +2 | sed "s/^/$p\t/"
}
RULES=${1:-all}
{
printf 'project\ta\tb\tnodes_raw\tcmp\twl\tcov\tnodes_norm\tcmp\twl\tcov\n'
run_one() { p=$1; shift; echo "$@" | $BIN "$RULES" $(F $p) | tail -n +2 | awk -v p=$p '{print p"\t"$0}'; }
run_one queue_worker "email_worker::classify image_worker::inspect"
run_one queue_worker "email_worker::retry_delay_seconds image_worker::next_delay"
run_one telemetry_workspace "file_agent::persist_with_retries http_agent::send_with_retry"
run_one telemetry_workspace "file_agent::clean_line http_agent::normalize_body"
run_one telemetry_workspace "file_agent::ingest_lines http_agent::handle_request"
run_one telemetry_workspace "file_agent::file_timestamp http_agent::received_at"
run_one backup_sync "scanner::to_backup_key sync::remote_key_for"
run_one backup_sync "checksum::rolling_checksum sync::upload_in_chunks"
run_one spelling_variants "left::retry_while right::retry_for"
run_one spelling_variants "left::lookup_match right::lookup_if_let"
run_one spelling_variants "left::react_chain right::react_match"
run_one spelling_variants "left::cached_and right::cached_nested"
run_one spelling_variants "left::flush_all right::retry_for"
run_one spelling_variants "left::lookup_match right::evict_if_let"
run_one spelling_variants "left::store_try right::store_match"
} | column -t -s "$(printf '\t')"
