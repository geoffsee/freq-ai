---
name: issue-tracking
description: Guidance for creating GitHub issues and trackers, deciding when to comment on issues, and keeping issue/PR handoff notes high-signal.
---

# Issue Tracking

Use this skill when you are creating, editing, closing, or working from GitHub issues.

## Issue Creation Contract

Create GitHub issues only when they represent schedulable work or a deliberate workflow artifact.

- Schedulable work issues must have a clear outcome, acceptance criteria, verification steps, and at least one workflow or area label.
- Workflow artifact issues such as retrospectives, roadmap documents, housekeeping reports, strategic reviews, release checkpoints, and stakeholder updates are records, not schedulable work. Label them only with their artifact label unless the prompt explicitly says otherwise.
- Do not add `tracker` to an issue unless the issue is a parent or epic that groups child issues for execution.

## Tracker Contract

The `tracker` label is reserved for parent issues that the automation can execute.

A tracker issue must contain all of the following:

- The `tracker` label plus the workflow label it groups, for example `tracker,sprint` or `tracker,security`.
- A dependency hierarchy or equivalent ordered plan.
- An actionable checklist with child issue references in parser-compatible rows, for example `- [ ] #42 Title (blocked by #40)`.
- Child issue bodies linked back with `Tracked by #<tracker>`.

Do not use `tracker` for retrospective reports, housekeeping audit logs, single feature issues, child issues, or documents that merely mention other issues. If a workflow needs to record tracker maintenance, write that in the artifact body without adding the `tracker` label.

## Rule of Thumb

Comment on the issue when the written record materially changes what a future reader would do.

If the detail lives only in your head, in transient tool output, or in a rejected path, and someone picking up the issue tomorrow would make a worse decision without it, write it down.

If the information is already obvious from the diff, commits, or PR description, do not repeat it on the issue.

## Comment When One of These Triggers Fires

- Scope or acceptance criteria changed, or need clarification.
- A non-obvious technical decision was made or rejected, and the reason matters.
- An assumption turned out to be false, or an earlier plan was invalidated.
- A blocker or external dependency now controls the next step.
- You are pausing, handing off, or stopping at a non-obvious boundary.

## Anti-patterns

Do not comment just to:

- Narrate every commit.
- Restate the diff.
- Log routine progress like "investigating", "implemented", or "tests passed".
- Copy the same change summary that belongs in the PR description.

## Preferred Homes

- Issue comment: decisions, scope changes, blockers, invalidated assumptions, and handoff context.
- PR description: change summary, reviewer context, and test summary.
- Commit message: atomic code history.

Change summaries belong in the PR description, not the issue, unless the issue also needs a durable decision record or handoff note.

## Writing the Comment

- Start with the change in understanding or the consequence for the next person.
- Link issue, PR, or commit numbers only when they help someone resume work.
- Include the minimum context needed to continue.
- Call out what is still open, if anything.

## Quick Check Before You Close Out the Issue

- Would tomorrow's assignee choose a different next step without this comment?
- Is this comment recording one of the five triggers above?
- Is the PR description already the better home for this information?
