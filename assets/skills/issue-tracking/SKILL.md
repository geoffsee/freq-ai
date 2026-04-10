---
name: issue-tracking
description: Guidance for when agents should comment on GitHub issues, what belongs in the issue versus the PR description, and how to leave high-signal handoff notes.
---

# Issue Tracking

Use this skill when you are working on a GitHub issue and need to decide whether the issue itself needs a comment.

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
