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
    server.rs          HTTP server for "serve" subcommand
  custom_themes.rs     Theme definitions
```

## Platform-Specific Implementation
The codebase uses `#[cfg(target_arch = "wasm32")]` guards for platform-specific implementations. Examples include `assets.rs`, `config_store.rs`, and `shell.rs`, ensuring proper behavior across desktop (non-wasm32) and web (wasm32) builds.
