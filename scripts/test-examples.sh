#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "${repo_root}"

# `freq-ai-agent-runtime`'s build script requires Bun. Locate it on PATH or via
# `$BUN`; if neither is set, install into `~/.bun` and export `BUN` for the
# build script to pick up. Without this the `cargo test` invocation below
# panics on systems (e.g. fresh CI runners) where Bun is not preinstalled.
if [ -z "${BUN:-}" ] && ! command -v bun >/dev/null 2>&1; then
  echo "==> installing bun (not found on PATH)"
  curl -fsSL https://bun.sh/install | bash >&2
  BUN="${HOME}/.bun/bin/bun"
  export BUN
  export PATH="${HOME}/.bun/bin:${PATH}"
fi

echo "==> cargo test --workspace"
cargo test --workspace --quiet

echo "==> building dummy-agent binary"
cargo build -q -p dummy-agent --bin freq-ai-dummy-agent

export PATH="${repo_root}/target/debug:${PATH}"

run_example() {
  local name="$1"
  echo "==> example: ${name}"
  cargo run -q --example "${name}" -p dummy-agent
}

run_example argv_shapes
run_example spawn_dummy

echo "==> all checks passed"
