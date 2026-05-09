# Factory Backlog

Live tracking document for the active factory cycle. Updated by autonomous
workflows and humans as work progresses. Pair with `STATUS.md` for the
high-level cycle scope.

## Active Tracker: #51 — Factory Backlog: Baseline reliability, test coverage, and operability

### Cycle Goal

Establish a reliable foundation: harden the snapshot generation runtime
paths, add cross-adapter smoke tests, and initialize the factory's live
tracking documents.

### Task Dependency Hierarchy

| Issue | Title | Depends On | Depended On By | Layer | Status |
|-------|-------|-----------|----------------|-------|--------|
| #48 | Audit and extend Tokio runtime-context handling in snapshot generation | — | — | 0 | 🔴 Not Started |
| #49 | Add launch-path smoke tests for each agent adapter | — | — | 0 | 🔴 Not Started |
| #50 | Initialize ISSUES.md and STATUS.md as live factory tracking documents | — | — | 0 | ✅ Done |

All three items are independent (Layer 0) and can be executed in parallel.

### Layer 0 — Validation Gate

All three items must satisfy their own acceptance criteria **and** the
following before the layer is considered complete:

- `cargo test --workspace` green (covers #48 and #49)
- `cargo clippy --workspace --all-targets -- -D warnings` clean (covers #48 and #49)
- `ls ISSUES.md STATUS.md` succeeds and grep assertions pass (covers #50)
- No new test failures relative to `master` at sprint start

The sprint is complete when all three checkboxes are checked and CI passes
on `master`.

### Checklist

- [ ] #48 Audit and extend Tokio runtime-context handling in snapshot generation
- [ ] #49 Add launch-path smoke tests for each agent adapter
- [x] #50 Initialize ISSUES.md and STATUS.md as live factory tracking documents

### Fallback / Rollback Rules

| Scenario | Action |
|----------|--------|
| #48 runtime fix causes Clippy regression | Revert the offending change in a follow-up commit; keep test scaffolding |
| #49 adapter tests introduce flaky failures | Mark flaky test `#[ignore]` with a comment explaining the condition; open a follow-up issue |
| #50 file format incompatible with housekeeping parser | Adjust format to match parser expectation; re-verify with `grep` assertions |
| CI fails after merge of any item | Revert the merge commit; investigate and re-open the child issue |

No database migrations, no API changes, no feature-flag toggles — all
changes are additive or test-only, so rollback is a simple revert.

## Software Factory Setup Backlog

Actionable next steps captured outside the current tracker. Items here
graduate into a tracker when picked up.

- _(empty — populate as new factory readiness gaps are identified)_

## Maintenance Notes

- This file is consumed verbatim by `housekeeping`, `autopilot`, and
  `tracker-loop-dispatch` workflows; keep section headings stable.
- When a tracker closes, archive its hierarchy/checklist below a
  `## Archived Trackers` heading rather than deleting it, so historical
  context survives.
