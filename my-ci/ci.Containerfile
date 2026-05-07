# syntax=docker/dockerfile:1.7
#
# CI image for freq-ai.
#
# Bundles every dependency needed to run `cargo test --workspace --all-targets
# --locked` and the integration tests under `crates/cli/tests/`:
#
#   * Rust stable toolchain (rustup-managed, via the rust:bookworm base).
#   * GTK / WebKit / appindicator / xdo system libs required by the Dioxus
#     desktop UI crate to compile.
#   * Node.js + Bun, used by `crates/agent-runtime` (`bun install` step in CI).
#   * GitHub CLI (`gh`) — required because the CLI integration tests shell out
#     to `gh` even on dry-run code paths; without it on $PATH the binary
#     either dies early (cmd_stdout_or_die) or pollutes test output with
#     `gh: To use GitHub CLI in a GitHub Actions workflow, set the GH_TOKEN`
#     warnings (see issue history for the failures this fixes).
#   * pkg-config / build-essential / curl / git / ca-certificates — generic
#     build prerequisites that other transitive crates expect.
#
# The image is intentionally agnostic of GH_TOKEN: tests should not require it.
# A consumer that wants gh to actually authenticate can pass the token via the
# GH_TOKEN env var at run time.
FROM rust:bookworm

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_TERM_COLOR=always \
    CARGO_INCREMENTAL=1 \
    CARGO_NET_RETRY=10 \
    RUSTUP_MAX_RETRIES=10 \
    RUST_BACKTRACE=short \
    PATH="/root/.bun/bin:${PATH}"

# Base system + Dioxus desktop deps (mirrors .github/workflows/ci.yml).
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        gnupg \
        pkg-config \
        libssl-dev \
        libgtk-3-dev \
        libwebkit2gtk-4.1-dev \
        libayatana-appindicator3-dev \
        libxdo-dev \
        unzip \
    && rm -rf /var/lib/apt/lists/*

# GitHub CLI (`gh`) from the official apt repo.
RUN install -dm 0755 /etc/apt/keyrings \
    && curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
        | tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
    && chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
        > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends gh \
    && rm -rf /var/lib/apt/lists/*

# Node.js (LTS) — required by `crates/agent-runtime` and used by Bun-managed
# scripts. NodeSource keeps a deterministic, well-known apt source.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Bun — `crates/agent-runtime/bun.lock` is the source of truth for agent CLIs.
RUN curl -fsSL https://bun.sh/install | bash

# Sanity-check that all the binaries we promise are actually on PATH.
RUN cargo --version \
    && rustc --version \
    && git --version \
    && gh --version \
    && node --version \
    && bun --version \
    && pkg-config --version

WORKDIR /app
