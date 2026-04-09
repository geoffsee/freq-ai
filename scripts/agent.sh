#!/usr/bin/env bash
set -euo pipefail


# agent.sh — Thin wrapper around the Rust-based dev agent (crates/dev)
#
# Usage:
#   ./scripts/agent.sh              # launches the GUI
#   ./scripts/agent.sh --auto       # launches GUI with auto-mode enabled
#   ./scripts/agent.sh --dry-run    # launches GUI with dry-run enabled

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

# bot creds
ENV_FILE_PATH="$ROOT/.env.agent"
set -a
source "${ENV_FILE_PATH}"
set +a

cargo run --quiet -- "$@"