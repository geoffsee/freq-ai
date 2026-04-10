You are planning the next XP iteration for the {{project_name}} project.

Read AGENTS.md and .agents/skills/ for project conventions.

## Human Feedback

{{feedback}}

## Instructions

Incorporate the feedback and produce the FINAL iteration plan.

Then create the sprint artifacts:

1. Create one GitHub issue per planned story.
2. Each story body must state:
   - the smallest shippable increment
   - the first failing-then-passing test or verification step
   - any pairing expectation
3. Create a tracker issue labeled `sprint,tracker` with the iteration goal, dependency table, and checklist.
4. Update each child issue with `Tracked by #<tracker>`.

Reject oversized stories before filing them. If `dry_run` is enabled, print the exact `gh` commands instead of running them.
