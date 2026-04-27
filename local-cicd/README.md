# local-cicd

Reproduces each `.github/workflows/*.yml` job locally in Docker. Useful for
iterating on a CI failure without pushing to a branch.

## How it stays in sync with real CI

Every job's command sequence lives in **`scripts/ci/*.sh`**. Both real CI and
the Dockerfiles in this directory call into those scripts. Don't add command
logic directly to a Dockerfile or to a workflow YAML — put it in a script.

| Service     | Mirrors                                          | Script                  |
|-------------|--------------------------------------------------|-------------------------|
| `compat`    | `.github/workflows/cli-compat.yml`               | `scripts/ci/compat.sh`  |
| `model-dig` | `.github/workflows/model-dig.yml`                | `scripts/ci/model-dig.sh` |
| `release`   | `.github/workflows/release.yml` (linux x86_64)   | inline `cargo build`    |
| `weekly`    | `.github/workflows/merge-and-release.yml`        | inline `codex exec`     |

`release` is single-arch — the real release matrix builds for mac/windows too,
which Docker can't reproduce.

## Usage

```sh
# Build a single job
docker compose -f local-cicd/compose.yml build compat

# Build everything (release waits on compat passing)
docker compose -f local-cicd/compose.yml build

# Run the weekly agent locally (needs API keys exported in your shell)
docker compose -f local-cicd/compose.yml run --rm weekly
```
