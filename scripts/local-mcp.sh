#!/bin/sh
# Launch the native stdio MCP adapter with local Cargo and no remote imports/storage.
set -eu

if [ "$#" -ne 0 ]; then
    echo 'Usage: scripts/local-mcp.sh (configure RSTYLE_BIN and RSTYLE_MCP_BIN in the environment)' >&2
    exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-target}
case "$target" in
    /*) ;;
    *) target="$root/$target" ;;
esac
analyzer=${RSTYLE_BIN:-$target/debug/cargo-rstyle}
server=${RSTYLE_MCP_BIN:-rstyle-mcp-server}

if [ ! -x "$analyzer" ] || [ ! -x "$(dirname -- "$analyzer")/rstyle-driver" ]; then
    echo 'Build cargo-rstyle and rstyle-driver together, then set RSTYLE_BIN to the cargo-rstyle executable.' >&2
    exit 2
fi
if ! command -v "$server" >/dev/null 2>&1; then
    echo 'Build rstyled/rstyle-mcp-server and set RSTYLE_MCP_BIN to its executable.' >&2
    exit 2
fi

# Do not inherit a hosted adapter configuration. The empty repository allowlist
# refuses repo_url imports before GitHub access; clients must pass a local path.
unset RSTYLE_GITHUB_APP_ID RSTYLE_GITHUB_INSTALLATION_ID RSTYLE_GITHUB_APP_KEY
unset RSTYLE_REPORTS_URL RSTYLE_ORIGIN_TOKEN RSTYLE_API_TOKEN
unset CUA_HOST_URL CUA_HOST_TOKEN CUA_CALLBACK_URL
unset RSTYLE_CUA_ENDPOINT RSTYLE_CUA_CHECKPOINT
export RSTYLE_BIN="$analyzer"
export RSTYLE_GITHUB_AUTH=cli
export RSTYLE_GITHUB_ALLOWED_REPOS=''

# No arguments are forwarded: --http would select the remote execution adapter.
# Startup messages belong on stderr; stdout is exclusively MCP JSON-RPC.
exec "$server"
