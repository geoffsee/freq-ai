#!/usr/bin/env bash
set -euo pipefail


# agent.sh — Thin wrapper around the Rust-based dev agent (crates/dev)
#
# Usage:
#   ./scripts/agent.sh              # launches the GUI in web mode (default)
#   ./scripts/agent.sh --desktop    # launches the GUI in desktop mode
#   ./scripts/agent.sh --auto       # launches GUI with auto-mode enabled
#   ./scripts/agent.sh --dry-run    # launches GUI with dry-run enabled

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

# bot creds
ENV_FILE_PATH="$ROOT/.env.agent"
set -a
source "${ENV_FILE_PATH}"
set +a

MODE="web"
ARGS=()

for arg in "$@"; do
    if [[ "$arg" == "--desktop" ]]; then
        MODE="desktop"
    else
        ARGS+=("$arg")
    fi
done

if [[ "$MODE" == "desktop" ]]; then
    cargo run --quiet -- "${ARGS[@]}"
else
    dx serve --platform web
fi