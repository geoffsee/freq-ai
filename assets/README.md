# Assets

Bundled agent skills and workflow definitions for freq-ai. This directory is
the compile-time source — skills are embedded via `rust-embed` and bootstrapped
into the target repo's `.agents/skills/` at runtime. Workflow templates are
read directly from `assets/workflows/{preset}/` at runtime.

This keeps the target repo's `.agents/` directory clean for its own agent configuration.

## Skills

Skills in `skills/` are embedded at compile time and written to the target repo's
`.agents/skills/` on first run (via `ensure_agent_files`).

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

Workflows are organized under `workflows/` in **preset** directories. Each preset is a named collection of workflows that appear as action buttons in the sidebar.

```
workflows/
  default/              <-- preset
    sprint-planning/    <-- workflow
      workflow.yaml
      draft.md
      finalize.md
    ideation/
      ...
  lean/                 <-- another preset (only the workflows you need)
    sprint-planning/
      workflow.yaml
      draft.md
      finalize.md
```

### Presets

A preset is a folder directly under `workflows/`. The sidebar loads whichever preset is selected and renders its workflows as action buttons. When only one preset exists, no selector is shown. When multiple presets exist, a dropdown appears in the sidebar.

To create a new preset, copy the `default/` directory and remove the workflows you don't need, or start from scratch with just the ones you want.

The active preset is stored in `dev.toml` as `workflow_preset` (defaults to `"default"`).

### Workflow files

Each workflow subdirectory contains:

- `workflow.yaml` — Metadata (name, display order, category, dependencies, context gatherer, phases)
- `draft.md` / `finalize.md` — Handlebars prompt templates with `{{variable}}` placeholders

To add a new workflow to a preset, create a subdirectory with a `workflow.yaml`. The sidebar picks it up on next launch.

### workflow.yaml reference

```yaml
name: Sprint Planning                 # display name in sidebar
id: sprint_planning                   # unique identifier
description: Plan the next sprint     # tooltip / documentation
pattern: two_phase                    # two_phase | one_shot | multi_round | implementation
context: sprint                       # context gatherer: sprint | strategic | retro | housekeeping | none
runner: my_custom_runner              # optional: named action from the Rust registry

ui:
  category: planning                  # groups buttons: discovery | planning | review | maintenance
  order: 50                           # sort order within category (lower = higher)
  visible: true                       # false to hide from sidebar
  requires_bot: false                 # true to disable button when no bot credentials

depends_on:                           # informational dependency graph
  - strategic_review

extra_context:                        # fetch GitHub issue bodies by label
  - name: report_synthesis
    label: uxr-synthesis

phases:
  draft:
    template: draft.md
    log_start: "Starting sprint planning draft..."
    log_complete: "Draft complete."
  finalize:
    template: finalize.md
    log_start: "Finalising sprint plan..."
    log_complete: "Sprint planning complete."

fragments:                            # reusable text blocks, included via {{> name}}
  my_fragment: |
    Shared text included with {{> my_fragment}}
```

### Default workflows

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
