#!/bin/sh
# Fast local tests: nextest + doctests (nextest skips doctests), same as CI.
# Reuses one compiled-deps cache across worktrees/clones via CARGO_TARGET_DIR
# (registry deps are keyed by content, so only workspace crates rebuild per worktree).
# Override: CARGO_TARGET_DIR=... scripts/test.sh. Extra args go to nextest.
set -e
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/rstyle/target}"
if ! cargo nextest --version >/dev/null 2>&1; then
  echo "cargo-nextest missing: cargo install cargo-nextest --locked" >&2
  exit 2
fi
cargo nextest run --workspace --locked "$@"
cargo test --workspace --locked --doc
