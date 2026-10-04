#!/bin/sh
# Builds the driver + CLI, then runs rstyle on this workspace. Extra args are passed through.
# Honors CARGO_TARGET_DIR (shared caches, CI volumes).
set -e
cd "$(dirname "$0")/.."
cargo build -q --workspace
exec "${CARGO_TARGET_DIR:-target}/debug/cargo-rstyle" rstyle "$@"
