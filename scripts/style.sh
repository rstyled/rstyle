#!/bin/sh
# Builds the driver + CLI, then runs rstyle on this workspace. Extra args are passed through.
set -e
cd "$(dirname "$0")/.."
cargo build -q --workspace
exec target/debug/cargo-rstyle rstyle "$@"
