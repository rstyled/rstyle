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
    cargo test --workspace --locked --no-run
    export RSTYLE_TEST_DRIVER="$(cd "$CARGO_TARGET_DIR/debug" && pwd)/rstyle-driver"
    test -x "$RSTYLE_TEST_DRIVER" || { echo "missing prebuilt driver: $RSTYLE_TEST_DRIVER" >&2; exit 1; }
    cargo nextest run --workspace --locked --profile ci "$@"
    exec cargo test --workspace --locked --doc
fi
exec cargo test --workspace --locked "$@"
