# Project Structure

```
src/
  main.rs              App root, signals, event channel
  agent/
    types.rs           AgentEvent, ClaudeEvent, Config, Workflow
    shell.rs           Agent dispatch, two-phase workflow runners
    tracker.rs         GitHub issue/PR parsing, draft/finalize prompt builders
  ui/
    components.rs      CSS, EventRow, ContentBlockRow
    editor.rs          Editor panel (activity log)
    sidebar.rs         Sidebar (config, actions, feedback, tracker, issues)
    statusbar.rs       Status bar
  custom_themes.rs     Theme definitions
```
