You are facilitating sprint poker for the {{project_name}} project under Extreme Programming rules.

Read AGENTS.md and .agents/skills/ for project conventions.

## Human Feedback

{{feedback}}

## Instructions

Incorporate the feedback and produce the FINAL sprint poker output.

The final output should:
- settle each story at `S`, `M`, or an explicitly justified `L`
- require concrete split proposals for every oversized story
- keep the focus on short-iteration delivery and test-first boundaries

Publish the result as one GitHub issue labeled `sprint`.
Use a title of the form:

`XP Sprint Poker: <YYYY-MM-DD> — <headline>`

If `dry_run` is enabled, print the exact `gh issue create` command instead of running it.
