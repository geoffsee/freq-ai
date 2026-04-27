#!/usr/bin/env bash
# Probes each provider CLI's --version (or equivalent) to detect upstream
# breakage before the heavier wrapper test suite runs. Mirrors the
# `Verify CLI Binaries` step in .github/workflows/cli-compat.yml.
#
# Writes a transcript to verify-logs/cli-compat-verify.log and exits non-zero
# on the first failing probe. Real CI uploads this log as an artifact and
# feeds it to the codex autofix job.
set -euo pipefail

mkdir -p verify-logs
log="verify-logs/cli-compat-verify.log"
: > "${log}"

run_logged() {
  echo "+ $*" | tee -a "${log}" >/dev/null
  # shellcheck disable=SC2068
  "$@" 2>&1 | tee -a "${log}"
}

run_logged freq-ai-dummy-agent --version
# v2+ rejects --version; keep a fallback for 1.x-style installs.
run_logged bash -lc "freq-ai-cli-compat-fixture version || freq-ai-cli-compat-fixture --version"
run_logged claude --version
# cline does not support --version; keep a compatibility fallback.
run_logged bash -lc "cline version || cline -v"
run_logged codex --version
run_logged copilot --version
run_logged gemini --version
# grok-cli version flags differ across releases.
run_logged bash -lc "grok --version || grok -v || grok version"
run_logged junie --version
