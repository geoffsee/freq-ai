# .agents - Agent Configuration

This directory contains agent skills and workflow definitions for the freq-ai project.

## Skills

Skills in `skills/` are automatically discovered by compatible agents (Claude Code, Cursor, VS Code Copilot, Gemini CLI, and others).

| Skill | Description |
| :--- | :--- |
| `project-context` | Core project context, priorities, and key resources |
| `architecture` | High-level system design and component overview |
| `coding-standards` | Rust coding patterns and conventions |
| `testing` | Test commands, verification workflow, and submission checklist |
| `user-personas` | Adopter personas for UXR synthesis |
| `issue-tracking` | GitHub issue/PR hygiene guidance |
| `code-explorer` | Use toak CLI for codebase snapshots and LLM context |

## Workflows

Workflow definitions in `workflows/` drive the sidebar actions and prompt templates. Each subdirectory contains:

- `workflow.yaml` — Metadata (name, display order, category, dependencies, context gatherer, phases)
- `draft.md` / `finalize.md` — Handlebars prompt templates with `{{variable}}` placeholders

To add a new workflow, create a new subdirectory with a `workflow.yaml`. The sidebar renders buttons dynamically from this directory structure.

| Workflow | Category | Pattern |
| :--- | :--- | :--- |
| `ideation` | discovery | two-phase |
| `report-research` | discovery | two-phase |
| `interview` | discovery | multi-round |
| `strategic-review` | planning | two-phase |
| `roadmapper` | planning | two-phase |
| `sprint-planning` | planning | two-phase |
| `code-review` | review | one-shot |
| `security-scan` | review | one-shot |
| `security-review` | review | one-shot |
| `retrospective` | review | two-phase |
| `housekeeping` | maintenance | two-phase |
| `refresh-agents` | maintenance | one-shot |
| `refresh-docs` | maintenance | one-shot |
