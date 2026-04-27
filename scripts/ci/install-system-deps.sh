#!/usr/bin/env bash
# Installs Linux system packages required to build freq-ai (Dioxus desktop on
# non-wasm targets needs GTK/WebKit) and to run the provider CLIs (libsecret for
# grok-cli's keytar). Called from .github/workflows/* and local-cicd/Dockerfile*.
set -euo pipefail

if ! command -v apt-get >/dev/null 2>&1; then
  echo "install-system-deps.sh: apt-get not found; skipping (non-Debian host)."
  exit 0
fi

SUDO=""
if [[ "$(id -u)" -ne 0 ]]; then
  SUDO="sudo"
fi

$SUDO apt-get update
$SUDO apt-get install -y --no-install-recommends \
  pkg-config \
  libglib2.0-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  libxdo-dev \
  librsvg2-dev \
  libssl-dev \
  libsecret-1-0 \
  ca-certificates \
  curl
