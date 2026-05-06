# Project Structure

freq-ai is a Cargo workspace. Shared enums and configuration types live in
`crates/cli-common`; each supported agent CLI has a small adapter crate; the main
application is `crates/cli` (package `freq-ai`).

```
crates/
  cli-common/src/lib.rs        Agent, Workflow, Config, events, shared structs
  agent-common/src/lib.rs      AgentCliAdapter trait and argv helpers
  claude/ cline/ codex/ ...    Provider-specific CLI adapter crates
  cli/
    src/main.rs                Thin binary entry point
    src/lib.rs                 CLI parsing, App signals, event channel, GUI wiring
    src/agent/
      adapter_dispatch.rs      Maps Agent values to adapter implementations
      actions.rs               Named one-shot action registry
      assets.rs                Embedded skill/workflow assets and app-data materialization
      cli.rs                   Config loading and project-name inference
      config_store.rs          OS keychain storage for secrets
      issue.rs                 Tracker loop and issue implementation flow
      refresh.rs               Refresh Docs / Refresh Agents runners
      run.rs                   Agent subprocess launch and event parsing
      tracker.rs               GitHub issue/PR parsing and prompt builders
      workflow.rs              YAML workflow loading, preset discovery, context fetches
      workflows.rs             Generic draft/finalize workflow runners
    src/ui/
      components.rs            Shared CSS, event rows, content block rendering
      editor.rs                Editor panel tabs: output, files, personas, security, interview, chat
      personas.rs              User Personas Studio persistence and UI
      security.rs              Local security scan model and panel
      sidebar.rs               Config, actions, feedback, tracker, issues, PRs
      statusbar.rs             Status bar
      server.rs                HTTP server for `freq-ai serve`
    src/custom_themes.rs       Theme definitions
assets/
  skills/                      Bundled skills embedded at compile time
  workflows/<preset>/...       Bundled workflow YAML and prompt templates
docs/                          Human-facing project documentation
```

## Runtime Shape

The desktop app uses Dioxus signals in `crates/cli/src/lib.rs` to coordinate the
sidebar, editor tabs, agent event stream, tracker state, chat/interview turns,
security findings, workflow presets, and persona skill path. Agent subprocesses
emit structured events into a Tokio channel; the editor renders those events and
tracks file interactions as they stream in.

Workflows are data-driven where possible. `workflow.yaml` files under
`assets/workflows/<preset>/` and project-local `.agents/workflows/<preset>/`
define sidebar metadata, prompt templates, and extra GitHub issue context.
Registered Rust actions handle flows that need custom execution, such as code
review, security review, refresh actions, chat, interviews, issue work, and PR
review fixes.

## Platform-Specific Implementation

The codebase uses `#[cfg(target_arch = "wasm32")]` guards for platform-specific
implementations. Examples include `config_store.rs`, `server.rs`, and UI panels
that perform filesystem writes only in desktop builds. Desktop mode can read and
write local persona JSON files; web mode exposes workflow inventory through the
HTTP server and leaves local persistence to desktop.
