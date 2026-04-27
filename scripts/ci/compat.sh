#!/usr/bin/env bash
# Top-level local-cicd entrypoint for the cli-compat job: install deps, build
# dummy-agent (so it lands on PATH), verify provider CLIs, then run the wrapper
# tests. Real CI splits these across separate steps for artifact handling, but
# the underlying sequence is identical.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

./scripts/ci/install-system-deps.sh
./scripts/ci/install-provider-clis.sh

# Compat fixture pinned at v2 (deliberate npm drill — v2 breaks --version).
npm install -g freq-ai-cli-compat-fixture@2.0.0

cargo build -p dummy-agent --bin freq-ai-dummy-agent
export PATH="${repo_root}/target/debug:${PATH}"

./scripts/ci/compat-verify.sh
./scripts/test-cli-compat.sh
