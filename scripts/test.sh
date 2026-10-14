#!/bin/sh
# Local tests against a shared CARGO_TARGET_DIR so worktrees/clones reuse compiled
# dependencies (only workspace crates rebuild per path). Pass --nextest to use
# the process-based runner; remaining arguments go to the selected runner.
# Override: CARGO_TARGET_DIR=... scripts/test.sh
set -e
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/rstyle/target}"
if [ "${1:-}" = "--nextest" ]; then
    shift
    cargo build --locked -p rstyle-driver
    export RSTYLE_TEST_DRIVER="$(cd "$CARGO_TARGET_DIR/debug" && pwd)/rstyle-driver"
    cargo nextest run --workspace --locked --profile ci "$@"
    exec cargo test --workspace --locked --doc
fi
exec cargo test --workspace --locked "$@"
