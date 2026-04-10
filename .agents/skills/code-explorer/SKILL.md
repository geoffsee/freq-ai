---
name: code-explorer
description: Use toak CLI to generate codebase snapshots for LLM context.
---

# Code Explorer (toak)

The `toak` CLI tokenizes git repositories into clean, LLM-friendly markdown. Use it when you need to explore unfamiliar parts of the codebase or get a full project overview.

## Generating a Codebase Snapshot

```bash
toak generate -d . -o snapshot.md --quiet
```

- `-d <dir>` — project root (default: `.`)
- `-o <path>` — output file (default: `prompt.md`)
- `--quiet` — suppress per-file token counts

This produces a single markdown file containing every tracked source file, cleaned of comments, imports, and secrets. Binary files, build artifacts, and files matching `.aiignore` are excluded automatically.

## Excluding Files

Create or edit `.aiignore` (same syntax as `.gitignore`) to exclude files from toak processing. Nested `.aiignore` files in subdirectories are also supported.

## When to Use

- **Starting a new issue** — a snapshot is already included in your prompt.
- **Cross-cutting changes** — generate a snapshot and search it for patterns.
- **Unfamiliar crate** — generate a snapshot of just that crate: `toak generate -d crates/compute-node -o compute.md --quiet`.
