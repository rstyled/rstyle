#!/bin/sh
set -eu

usage() {
    cat <<'EOF'
Usage: sh install.sh [--force]

Download, build, and install cargo-rstyle and rstyle-driver from GitHub.
Requires rustup and a native build toolchain; no local checkout is needed.

Options:
  --force   Replace an existing installation, even at the same version.
  -h, --help
            Show this help.

The pinned Rust nightly and its required components are installed automatically.
Binaries go to CARGO_INSTALL_ROOT, CARGO_HOME, or ~/.cargo (under bin/).
This script downloads source and compiles it locally, not prebuilt binaries.
EOF
}

force=false
for argument in "$@"; do
    case "$argument" in
        --force) force=true ;;
        -h|--help) usage; exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$argument" >&2; exit 2 ;;
    esac
done

if ! command -v rustup >/dev/null 2>&1; then
    printf '%s\n' 'rustup is required. Install it from https://rustup.rs, then rerun this script.' >&2
    exit 1
fi

# Keep this pin in sync with rust-toolchain.toml. A downloaded script has no checkout.
toolchain=nightly-2026-09-30
repository=https://github.com/rstyled/rstyle.git
install_root=${CARGO_INSTALL_ROOT:-${CARGO_HOME:-"$HOME/.cargo"}}

printf 'Installing Rust toolchain %s and compiler components...\n' "$toolchain"
rustup toolchain install "$toolchain" --profile minimal \
    --component rustc-dev \
    --component llvm-tools-preview \
    --component rust-src

set -- install --git "$repository" --locked --root "$install_root"
if [ "$force" = true ]; then
    set -- "$@" --force
fi

printf 'Downloading and building rstyle from %s...\n' "$repository"
rustup run "$toolchain" cargo "$@" cargo-rstyle rstyle-driver

printf '\nInstalled cargo-rstyle and rstyle-driver in %s/bin\n' "$install_root"
case ":${PATH:-}:" in
    *:"$install_root/bin":*) ;;
    *) printf 'Add %s/bin to PATH before running cargo rstyle.\n' "$install_root" ;;
esac
printf '%s\n' 'Run cargo rstyle from the Rust project you want to analyze.' \
    "Keep $toolchain installed; the driver needs its compiler libraries."
