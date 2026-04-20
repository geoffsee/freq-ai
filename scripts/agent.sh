#!/usr/bin/env bash
set -euo pipefail

# agent.sh — Thin wrapper around the Rust-based dev agent
#
# Usage:
#   ./scripts/agent.sh                       # launches API-backed web server (default)
#   ./scripts/agent.sh --dx                  # launch web UI via dioxus CLI (no embedded API)
#   ./scripts/agent.sh --desktop             # launches the native desktop GUI via cargo
#   ./scripts/agent.sh --serve               # launch API-backed web server explicitly

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

# bot creds
ENV_FILE_PATH="$ROOT/.env.agent"
set -a
source "${ENV_FILE_PATH}"
set +a

MODE="serve"
ARGS=()
PORT_SET=0

for arg in "$@"; do
    case "$arg" in
        --desktop)
            MODE="desktop"
            ;;
        --dx)
            MODE="dx"
            ;;
        --serve)
            MODE="serve"
            ;;
        *)
            ARGS+=("$arg")
            if [[ "$arg" == "--port" || "$arg" == --port=* ]]; then
                PORT_SET=1
            fi
            ;;
    esac
done

case "$MODE" in
    desktop)
        cargo run --quiet -- "${ARGS[@]}"
        ;;
    dx)
        dx serve --platform web
        ;;
    serve)
        if [[ "$PORT_SET" -eq 0 ]]; then
            cargo run --quiet -- serve --port 0 "${ARGS[@]}"
        else
            cargo run --quiet -- serve "${ARGS[@]}"
        fi
        ;;
    *)
        if [[ "$PORT_SET" -eq 0 ]]; then
            cargo run --quiet -- serve --port 0 "${ARGS[@]}"
        else
            cargo run --quiet -- serve "${ARGS[@]}"
        fi
        ;;
esac
