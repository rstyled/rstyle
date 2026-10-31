#!/bin/sh
# Run the existing workspace suite once, including its CLI/driver subprocesses.
set -eu
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/coverage}"
coverage_parent=$(dirname "$CARGO_TARGET_DIR")
mkdir -p "$coverage_parent"
coverage_parent=$(cd "$coverage_parent" && pwd)
CARGO_TARGET_DIR="$coverage_parent/$(basename "$CARGO_TARGET_DIR")"
export CARGO_TARGET_DIR
export CARGO_LLVM_COV_TARGET_DIR="$CARGO_TARGET_DIR"
# Coverage maps do not need full DWARF; keep line backtraces without duplicating
# the normal test build's large debug-symbol footprint on hosted runners.
export CARGO_PROFILE_DEV_DEBUG=line-tables-only
export CARGO_PROFILE_TEST_DEBUG=line-tables-only
# Never retain a report restored from the normal target cache after a failed run.
rm -f target/coverage/lcov.info
# Absolute output survives fixture cwd changes. LLVM locks and merges a bounded
# four-file pool per module, including repeated CLI/compiler subprocesses.
export LLVM_PROFILE_FILE_NAME='rstyle-%4m.profraw'
coverage_env=$(cargo llvm-cov show-env --sh --doctests)
eval "$coverage_env"
cargo llvm-cov clean --workspace
./scripts/test.sh --nextest "$@"
mkdir -p target/coverage
cargo llvm-cov report --doctests --lcov --output-path target/coverage/lcov.info
