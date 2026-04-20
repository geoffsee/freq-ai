# CLI Compat Autofix Skill

Use this skill when the `CLI Compat` workflow fails in `Verify CLI Binaries` and Codex is asked to generate a remediation patch.

## Goal

- Make CLI verification resilient across provider CLI versions.
- Keep checks strict enough to catch truly missing binaries.
- Minimize blast radius and avoid unrelated edits.

## Constraints

- Only edit files directly related to compatibility checks:
  - `.github/workflows/cli-compat.yml`
  - `scripts/test-cli-compat.sh`
  - provider wrapper crates under `crates/*/src/wrapper.rs`
  - related live-compat tests under `crates/*/tests/live_cli.rs`
- Prefer fallbacks that preserve current behavior for known-good versions.
- Do not change unrelated logic, formatting, or architecture.

## Validation

- Run the smallest relevant validation commands for changed files.
- Prefer targeted crate tests over full-workspace runs.
- Confirm no merge markers or malformed YAML were introduced.

## Output

- Apply edits directly to the working tree; do not commit.
- Keep changes concise and focused.

## Commit Message Guidance

When asked to draft a commit message from staged changes:

- Use imperative mood and concise scope (for example: `fix(ci): ...`).
- First line must be <= 72 chars.
- Optional body should explain why, not just what.
- Output plain text only (no markdown fences or JSON).
