#!/usr/bin/env bash
# Installs every provider CLI freq-ai wraps. Used by both real CI
# (.github/workflows/cli-compat.yml, model-dig.yml) and local-cicd/Dockerfile*.
# Single source of truth so the two cannot drift.
set -euo pipefail

if ! command -v npm >/dev/null 2>&1; then
  echo "install-provider-clis.sh: npm not found on PATH" >&2
  exit 1
fi

npm install -g \
  @anthropic-ai/claude-code \
  @openai/codex \
  @github/copilot \
  @google/gemini-cli \
  cline \
  grok-cli

# Junie ships its own installer (jb-junie binary in ~/.local/bin).
curl -fsSL https://junie.jetbrains.com/install.sh | bash

# Some installer versions land the binary as `jb-junie`; symlink for parity.
if [[ -x "${HOME}/.local/bin/jb-junie" && ! -e "${HOME}/.local/bin/junie" ]]; then
  ln -s "${HOME}/.local/bin/jb-junie" "${HOME}/.local/bin/junie"
fi
