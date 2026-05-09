# Project Status

Live, single-page snapshot of the freq-ai project. Pair with `ISSUES.md`
for the active factory tracker and dependency hierarchy.

## Current Cycle

- **Active tracker:** #51 — Factory Backlog: Baseline reliability, test coverage, and operability
- **Cycle scope:** active — harden snapshot-generation runtime paths, add cross-adapter smoke tests, and initialize live factory tracking documents (`ISSUES.md`, `STATUS.md`).
- **Layer 0 in flight:** #48 (runtime-context audit), #49 (adapter smoke tests), #50 (live tracking docs).

## Capability Tracking

| Capability | Tracking Issue | Status |
|------------|----------------|--------|
| Live factory tracking documents (ISSUES.md / STATUS.md) | #50 | ✅ Done |
| Snapshot Tokio runtime-context hardening | #48 | 🔴 Not Started |
| Cross-adapter launch-path smoke tests | #49 | 🔴 Not Started |

## Software Factory Charter

The CI Governance workflow is governed by `CHARTER.md`. The remainder of
the factory cycle (`autopilot`, `tracker-loop-dispatch`,
`weekly-backlog-curation`, `autonomous-sprint`, `factory-cycle-dispatch`,
`release-mediator`, `release-tag-publisher`, `release`,
`nightly-housekeeping`) operates per `.github/CI.md` and is out of scope
of `CHARTER.md`. `COVENANT.md` applies universally.

## Maintenance Notes

- This file is consumed by `housekeeping`, `autopilot`, and
  `tracker-loop-dispatch`; keep the `## Current Cycle` section present so
  drift sweeps can locate the active scope line.
- When a capability ships, flip its row to ✅ and leave the row in place
  for at least one cycle so housekeeping can confirm parity with GitHub
  issue state before the row is archived.
