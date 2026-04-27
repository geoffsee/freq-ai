# Merge and release

Use this skill when the `Merge & release (on demand)` workflow runs. You drive `gh` and `git` in the checked-out repository to merge open PRs, bump the workspace crate version, push `master`, and push a release tag.

## Environment

- `GH_TOKEN` is set; use `gh` for all GitHub operations.
- Default branch name is in env `DEFAULT_BRANCH` (for example `master`). Use it for merges and local sync.
- Release tags must match `v*` (for example `v0.7.9`) so the existing `Release` workflow runs on tag push.

## Steps

1. **Sync local default branch**

   ```bash
   git fetch origin
   git checkout "$DEFAULT_BRANCH"
   git pull --ff-only "origin/$DEFAULT_BRANCH"
   ```

2. **Merge open PRs (simple pass)**

   - List: `gh pr list --state open --base "$DEFAULT_BRANCH" --json number,isDraft,mergeable,mergeStateStatus`
   - Skip drafts.
   - Prefer PRs that are `MERGEABLE` and not `DIRTY` / blocked if `mergeStateStatus` indicates problems; skip ones you cannot merge cleanly.
   - Merge with a normal merge commit: `gh pr merge <number> --merge --delete-branch` (omit `--delete-branch` if it fails for policy reasons).
   - After each successful merge: `git pull --ff-only "origin/$DEFAULT_BRANCH"` before the next.
   - If nothing is mergeable, continue to versioning anyway.

3. **Bump workspace version**

   - In the repo root `Cargo.toml`, under `[workspace.package]`, increment **patch** only (e.g. `0.7.8` → `0.7.9`). Edit that single version line.
   - Refresh the lockfile: `cargo generate-lockfile`

4. **Commit and push**

   - Stage `Cargo.toml` and `Cargo.lock`.
   - Commit, for example: `chore: bump workspace version to X.Y.Z`
   - `git push origin "$DEFAULT_BRANCH"`

5. **Tag and push (triggers release)**

   - `git tag -a "vX.Y.Z" -m "vX.Y.Z"` using the same version as in `Cargo.toml`.
   - `git push origin "vX.Y.Z"`

## Rules

- Do not change application logic, dependencies for features, or unrelated files.
- If `cargo generate-lockfile` or a merge fails, stop and report the error in your final output; do not force-push or rewrite history.
- Do not create duplicate tags; if the tag already exists remotely, skip tagging and say so.
