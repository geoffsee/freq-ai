# Project — Agent Instructions

This project is a software application. Your goal is to help build, maintain, and expand the codebase.

## Skills

Project-specific agent skills are available in `.agents/skills/`. Load them when relevant:

- `project-context` — Core project priorities and key resources
- `architecture` — High-level system design and component overview
- `coding-standards` — Project-specific coding conventions and patterns
- `user-personas` — Adopter personas for UXR synthesis
- `issue-tracking` — Guidance on GitHub issue/PR hygiene
- `testing` — Test commands and verification workflow
- `code-explorer` — Use toak CLI for codebase snapshots and LLM context

## Workflows

Workflow definitions live in `.agents/workflows/`. Each subdirectory contains a
`workflow.yaml` (metadata, display order, dependencies) and Handlebars prompt
templates (`draft.md`, `finalize.md`, etc.).

The sidebar renders action buttons directly from this directory — add a new
workflow by creating a new subdirectory with a `workflow.yaml`. The `ui.category`
field controls grouping (discovery, planning, review, maintenance) and `ui.order`
controls sort position within a category.

Two-phase workflows (draft → human feedback → finalize) are driven entirely by
YAML config + templates via the generic `run_workflow_draft` / `run_workflow_finalize`
runners. Workflows with complex execution logic (interview, code-review, work-on-issue)
keep dedicated Rust runners but can still externalize their prompt templates.

Template variables use `{{variable}}` syntax (Handlebars). Context data is gathered
automatically based on the `context` field in `workflow.yaml` (sprint, strategic,
retro, housekeeping). Extra GitHub issue lookups are declared via `extra_context`
entries that fetch issue bodies by label.

## Key Rules

- Always verify your changes by running the appropriate tests.
- When working on a GitHub issue, follow `.agents/skills/issue-tracking/` for high-signal updates.
- Adhere to the project's coding standards and architectural principles.

## Label Conventions

All GitHub issue labels are declared in `.github/labels.yml` (source of truth) and
exposed as constants in `src/agent/tracker.rs` → `pub mod labels`.

### Workflow labels (no prefix)

| Label | When to apply |
|---|---|
| `tracker` | Parent/epic issue grouping children — **required** on every tracker |
| `ideation` | Issues filed by the Ideation workflow |
| `uxr-synthesis` | Issues filed by the UXR Synth workflow |
| `strategic-review` | The single living strategic-review issue produced by the Strategic Review workflow |
| `roadmap` | The single living roadmap issue produced by the Roadmapper workflow |
| `sprint` | Issues filed by the Sprint Planning workflow |
| `code-review` | Issues filed by the Code Review workflow |
| `security` | Issues filed by the Security Code Review workflow |
| `retrospective` | The single living retrospective issue produced by the Retrospective workflow |
| `dev-ui` | Related to the dev UI tool (`crates/dev`) — workflow-equivalent alias |

### Namespaced labels

| Prefix | Purpose | Examples |
|---|---|---|
| `area:` | Crate / subsystem | `area:edge-node`, `area:freq-cli`, `area:ci` |
| `kind:` | Type of work | `kind:bug`, `kind:feature`, `kind:refactor`, `kind:chore` |
| `severity:` | Security / bug severity (ordered) | `severity:critical` > `high` > `medium` > `low` > `info` |
| `priority:` | Sprint scheduling | `priority:p0` (must ship) → `priority:p3` (backlog) |
| `status:` | State beyond open/closed (rare) | `status:blocked`, `status:needs-review` |

### Composition rules

- Every issue should carry **at least one** workflow label or `area:` label (preferably both).
- Tracker issues carry `tracker` + the workflow label they group (e.g. `tracker,sprint`).
- Auto-filed findings (`security`, `code-review`, etc.) **must** include the workflow label.
- `severity:` is **required** on `security` and `kind:bug` issues.
- `priority:` is optional but recommended on tracker children.
- `dev-ui` is kept as a workflow-equivalent label; use `area:` for crates without a workflow label.

### In code

Label constants live in `src/agent/tracker.rs::labels` and are used by Rust code
that calls `gh` programmatically. Prompt templates in `.agents/workflows/` use
literal label strings (e.g. `--label "strategic-review"`) since those are
instructions to the AI agent, not compiled code.

### Operator contract: keeping live labels in sync

`.github/labels.yml` is the source of truth. The live GitHub repo can drift if a
new workflow label is added in code without being created on GitHub — `gh issue
create --label new-label` will then fail with a label-not-found error.

To add a new workflow label:

1. Add the entry to `.github/labels.yml` (name + description + color).
2. Sync it live, either by:
   - Running `.github/scripts/sync-labels.sh --apply` (requires `yq` and `gh`,
     idempotent — uses `--force` so it updates existing labels in place), OR
   - Running `gh label create "<name>" --description "..." --color "..." --force`
     directly if you only need to add one or two labels.
3. Add the constant to `src/agent/tracker.rs::labels` (the `pub mod
   labels` block at the top of the file).
4. Reference the constant via `labels::*` in any prompt builder that emits the
   new label — never hardcode the string.

To rename or retire a workflow label, do steps 1–3 in the opposite order so live
issues never reference a label that no longer exists.

### Operator contract: artifact discovery between workflows

Workflows that consume an upstream artifact (UXR Synth → Ideation, Strategic
Review → UXR Synth, Roadmapper → Strategic Review) discover that artifact by
**canonical label**, not by file path or title search. Extra context fetches are
declared in each workflow's `workflow.yaml` under `extra_context`:

```yaml
extra_context:
  - name: report_synthesis
    label: uxr-synthesis
```

The generic runner calls `fetch_issue_by_label` (in `src/agent/workflow.rs`)
which uses the same shape:

```
gh issue list --label <canonical-label> --state open --limit 1 --json number,title,body
```

If you add a new artifact-producing workflow, add a matching `extra_context`
entry in the consuming workflow's `workflow.yaml`. Do not introduce
title-keyword search — that path was deprecated in favour of label filtering
(#88) and the test guard in `tracker.rs::find_tracker_uses_label_filter_not_title_search`
exists to prevent regressions.

## Amendments

<!-- Add project-specific overrides, clarifications, or temporary rules below. -->
<!-- These take precedence over the general rules above. -->
