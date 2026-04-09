#!/usr/bin/env bash
set -euo pipefail

REPO="geoffsee/freq-ai"
INSTALL_SCRIPT="https://raw.githubusercontent.com/${REPO}/master/install.sh"

bold="\033[1m"
dim="\033[2m"
green="\033[32m"
red="\033[31m"
reset="\033[0m"

pass=0
fail=0

check() {
  local label="$1"; shift
  if "$@" >/dev/null 2>&1; then
    printf "  ${green}✓${reset} %s\n" "$label"
    ((pass++))
  else
    printf "  ${red}✗${reset} %s\n" "$label"
    ((fail++))
  fi
}

check_output() {
  local label="$1" pattern="$2"; shift 2
  local out
  if out=$("$@" 2>&1) && echo "$out" | grep -qE "$pattern"; then
    printf "  ${green}✓${reset} %s\n" "$label"
    ((pass++))
  else
    printf "  ${red}✗${reset} %s  ${dim}(expected /%s/)${reset}\n" "$label" "$pattern"
    ((fail++))
  fi
}

# ── install via remote script ───────────────────────────────────────
INSTALL_DIR="$(mktemp -d)/freq-ai-smoke"
trap 'rm -rf "$INSTALL_DIR"' EXIT
export FREQ_AI_INSTALL_DIR="$INSTALL_DIR"

BIN="$INSTALL_DIR/freq-ai"

printf "\n${bold}freq-ai smoke test${reset}\n\n"
printf "${dim}install script: %s${reset}\n" "$INSTALL_SCRIPT"
printf "${dim}install dir:    %s${reset}\n\n" "$INSTALL_DIR"

check "remote install script runs successfully" \
  bash -c "curl -fsSL '$INSTALL_SCRIPT' | bash"

# ── post-install checks ────────────────────────────────────────────
check "binary exists and is executable"           test -x "$BIN"
check_output "--help exits 0 and shows usage"     "USAGE|Usage|usage|freq-ai" "$BIN" --help
check_output "--version prints version"           "freq-ai [0-9]" "$BIN" --version

for cmd in gui ideation code-review security-review sprint-planning retrospective roadmapper housekeeping refresh-agents refresh-docs; do
  check "$cmd --help exits 0" "$BIN" "$cmd" --help
done

# ── summary ─────────────────────────────────────────────────────────
total=$((pass + fail))
printf "\n${bold}%d/%d passed${reset}" "$pass" "$total"
if [ "$fail" -gt 0 ]; then
  printf "  ${red}(%d failed)${reset}\n\n" "$fail"
  exit 1
else
  printf "  ${green}all good${reset}\n\n"
fi