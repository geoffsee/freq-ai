---
name: code-explorer
description: Use toak CLI to generate codebase snapshots and perform semantic search across the repository.
---

# Code Explorer (toak)

The `toak` CLI tokenizes git repositories into clean, LLM-friendly markdown and builds semantic search indexes. Use it when you need to explore unfamiliar parts of the codebase or find related implementations.

## Generating a Codebase Snapshot

```bash
toak generate -d . -o snapshot.md --quiet
```

- `-d <dir>` — project root (default: `.`)
- `-o <path>` — output file (default: `prompt.md`)
- `--quiet` — suppress per-file token counts

This produces a single markdown file containing every tracked source file, cleaned of comments, imports, and secrets. Binary files, build artifacts, and files matching `.aiignore` are excluded automatically.

## Semantic Search

After running `toak generate`, an `embeddings.json` database is also created. Query it with:

```bash
toak search "wire protocol message handling" -n 10
toak search "quota enforcement" --full
```

- First positional arg is the query string
- `-f <path>` — embeddings file (default: `embeddings.json`)
- `-n <count>` — number of results (default: 5)
- `--full` — show complete file content, not just a preview

## Excluding Files

Create or edit `.aiignore` (same syntax as `.gitignore`) to exclude files from toak processing. Nested `.aiignore` files in subdirectories are also supported.

## When to Use

- **Starting a new issue** — a snapshot is already included in your prompt. Use `toak search` for targeted lookups if needed.
- **Cross-cutting changes** — search for all code touching a concept (e.g., `toak search "tenant isolation"`).
- **Unfamiliar crate** — generate a snapshot of just that crate: `toak generate -d crates/compute-node -o compute.md --quiet`.
