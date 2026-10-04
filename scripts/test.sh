#!/bin/sh
# Local tests against a shared CARGO_TARGET_DIR so worktrees/clones reuse compiled
# dependencies (only workspace crates rebuild per path). Extra args go to `cargo test`.
# Override: CARGO_TARGET_DIR=... scripts/test.sh
set -e
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/rstyle/target}"
exec cargo test --workspace --locked "$@"
