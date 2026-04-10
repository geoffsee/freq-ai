//! # freq-ai
//!
//! Library entry point for the freq-ai dev agent. The standalone `freq-ai`
//! binary is a thin shim around [`run_with_overrides`]; library consumers
//! (e.g. project-specific shims that want to inject a custom skill layout)
//! should call [`run_with_overrides`] with a closure that mutates the
//! [`Config`] before dispatch.
//!
//! ## Example
//!
//! ```no_run
//! use freq_ai::SkillPaths;
//!
//! fn main() {
//!     freq_ai::run_with_overrides(|config| {
//!         config.skill_paths = SkillPaths {
//!             user_personas: ".agents/skills/freq-cloud-user-personas/SKILL.md".into(),
//!             issue_tracking: ".agents/skills/freq-cloud-issue-tracking/SKILL.md".into(),
//!         };
//!         config.bootstrap_agent_files = false;
//!     });
//! }
//! ```
#![allow(non_snake_case)]

pub mod agent;
pub mod custom_themes;
pub mod ui;

pub use agent::types::{Agent, Config, SkillPaths};

use agent::config_store::{
    clear_bot_private_key_pem, clear_bot_token, clear_local_inference_api_key,
    store_bot_private_key_pem, store_bot_token, store_local_inference_api_key,
};
use agent::actions::{ActionContext, lookup_action};
use agent::shell::{
    clear_stop_request, parse_args, preflight, request_stop, run_code_review,
    run_interview_draft, run_interview_respond, run_loop, run_pr_review_fix, run_refresh_agents,
    run_refresh_docs, run_security_code_review, run_single_issue, run_workflow_draft,
};
use agent::workflow::load_sidebar_entries;
use agent::tracker::{
    DEFAULT_REVIEW_BOT_LOGIN, PendingIssue, PrSummary, TrackerInfo, current_branch_pr,
    enable_auto_merge, fetch_unresolved_thread_counts, find_tracker, get_tracker_body,
    is_auto_merge_enabled, list_open_prs, open_pr_map_from, parse_pending,
};
use agent::types::{
    save_dev_config, AgentEvent, BotAuthMode, ChangedFile, ClaudeEvent, ContentBlock, EVENT_SENDER,
    FileChangeKind, InterviewTurn, Workflow,
};
use clap::{Parser, Subcommand};
use custom_themes::Theme;
use dioxus::prelude::*;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;
use ui::components::BASE_CSS;
use ui::security::{SecurityFinding, run_security_scan};
use ui::{Editor, Sidebar, Statusbar};

#[derive(Parser)]
#[command(
    name = "freq-ai",
    about = "Distributed application runtime agent",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, default_value = "claude")]
    agent: agent::types::Agent,

    #[arg(long)]
    auto: bool,

    #[arg(long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the GUI (default)
    Gui,
    /// Address review threads on a PR
    FixPr { pr: u32 },
    /// Run ideation draft
    Ideation,
    /// Run UXR synthesis draft
    UxrSynth,
    /// Run strategic review draft
    StrategicReview,
    /// Run roadmapper draft
    Roadmapper,
    /// Run sprint planning draft
    SprintPlanning,
    /// Run retrospective draft
    Retrospective,
    /// Run housekeeping draft
    Housekeeping,
    /// Run user interview
    Interview,
    /// Run code review
    CodeReview,
    /// Run security code review
    SecurityReview,
    /// Refresh agent files
    RefreshAgents,
    /// Refresh project documentation
    RefreshDocs,
    /// Run a single issue
    Issue { number: u32 },
    /// Run the main loop for a tracker
    Loop { tracker: u32 },
}

/// Standalone entry point — equivalent to `run_with_overrides(|_| {})`.
/// Used by the `freq-ai` binary.
pub fn run() {
    run_with_overrides(|_| {});
}

/// Library entry point for consumers that want to inject custom `Config`
/// fields (e.g. a project-specific skill layout) before the agent runs.
///
/// The closure receives a mutable `Config` populated from CLI args, env vars,
/// and `dev.toml`. Mutate it however you need; freq-ai then dispatches to
/// either the GUI or the requested CLI subcommand.
pub fn run_with_overrides<F>(overrides: F)
where
    F: FnOnce(&mut Config),
{
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let mut config = parse_args();
    config.agent = cli.agent;
    config.auto_mode = cli.auto;
    config.dry_run = cli.dry_run;
    overrides(&mut config);

    // Eagerly populate the issue-comment trigger cache from the (possibly
    // overridden) skill path so the sidebar reminder is non-empty before the
    // first render. Done after `overrides` so consumers' custom paths win.
    ui::sidebar::init_issue_comment_triggers(std::path::Path::new(
        &config.skill_paths.issue_tracking,
    ));

    match cli.command {
        Some(Commands::FixPr { pr }) => {
            run_pr_review_fix(&config, pr);
        }
        Some(Commands::Ideation) => run_workflow_draft(&config, "ideation"),
        Some(Commands::UxrSynth) => run_workflow_draft(&config, "report_research"),
        Some(Commands::StrategicReview) => run_workflow_draft(&config, "strategic_review"),
        Some(Commands::Roadmapper) => run_workflow_draft(&config, "roadmapper"),
        Some(Commands::SprintPlanning) => run_workflow_draft(&config, "sprint_planning"),
        Some(Commands::Retrospective) => run_workflow_draft(&config, "retrospective"),
        Some(Commands::Housekeeping) => run_workflow_draft(&config, "housekeeping"),
        Some(Commands::Interview) => run_interview_draft(&config),
        Some(Commands::CodeReview) => run_code_review(&config),
        Some(Commands::SecurityReview) => run_security_code_review(&config),
        Some(Commands::RefreshAgents) => run_refresh_agents(&config),
        Some(Commands::RefreshDocs) => run_refresh_docs(&config),
        Some(Commands::Issue { number }) => run_single_issue(&config, number),
        Some(Commands::Loop { tracker }) => run_loop(&config, tracker),
        Some(Commands::Gui) | None => {
            // Stash the finalised Config so the Dioxus App component can pick
            // it up via `parse_args` (which already loads from dev.toml). The
            // App's own use of `parse_args()` would otherwise lose the
            // overrides — but since the overrides are also persisted via the
            // explicit init_issue_comment_triggers call above, the only place
            // overrides matter inside the GUI is the next `parse_args` call,
            // which already reads `dev.toml`. Library consumers who need to
            // inject overrides that aren't expressible in `dev.toml` should
            // use the CLI subcommands instead of the GUI.
            CONFIG_OVERRIDE
                .set(config)
                .expect("CONFIG_OVERRIDE set twice");
            dioxus::launch(App);
        }
    }
}

/// Process-wide handoff for `run_with_overrides` → `App`. The Dioxus App
/// component reads from this on first render so library consumers' overrides
/// (e.g. custom `skill_paths`) survive into the GUI rather than being
/// re-derived from `dev.toml` alone.
static CONFIG_OVERRIDE: std::sync::OnceLock<Config> = std::sync::OnceLock::new();

#[component]
fn App() -> Element {
    let config = use_signal(|| {
        // Prefer the override stashed by `run_with_overrides` so any custom
        // `skill_paths` / `bootstrap_agent_files` survive into the GUI.
        CONFIG_OVERRIDE
            .get()
            .cloned()
            .unwrap_or_else(parse_args)
    });
    let mut tracker_ids = use_signal(Vec::<TrackerInfo>::new);
    let mut issues = use_signal(Vec::<PendingIssue>::new);
    let mut is_working = use_signal(|| false);
    let mut awaiting_feedback = use_signal(|| None::<Workflow>);
    let mut feedback_text = use_signal(String::new);
    let mut events = use_signal(Vec::<AgentEvent>::new);
    let mut changed_files = use_signal(Vec::<ChangedFile>::new);
    let mut pr_map_sig = use_signal(HashMap::<u32, u32>::new);
    let mut pull_requests = use_signal(Vec::<PrSummary>::new);
    let mut security_findings = use_signal(Vec::<SecurityFinding>::new);
    let mut interview_turns = use_signal(Vec::<InterviewTurn>::new);
    let mut interview_active = use_signal(|| false);
    let mut interview_done = use_signal(|| false);
    let mut interview_agent_buf = use_signal(String::new);
    let mut settings_status = use_signal(|| None::<String>);
    let root_sig = use_signal(|| config.read().root.clone());
    let mut auto_merge_enabled = use_signal(|| false);
    let expand_all = use_signal(|| false);
    let follow_mode = use_signal(|| true);
    let bottom_el = use_signal(|| None::<std::rc::Rc<MountedData>>);
    let mut theme = use_signal(Theme::tokyo_night);
    let workflow_entries = use_signal(|| load_sidebar_entries(&config.read().root));

    use_effect(move || {
        preflight(&config.read());

        // Initialize channel if not already done
        if EVENT_SENDER.get().is_none() {
            let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();
            let _ = EVENT_SENDER.set(tx);

            // Initialize auto-merge state from actual PR
            spawn(async move {
                let enabled = tokio::task::spawn_blocking(|| {
                    current_branch_pr()
                        .map(|pr| is_auto_merge_enabled(pr.number))
                        .unwrap_or(false)
                })
                .await
                .unwrap_or(false);
                auto_merge_enabled.set(enabled);
            });

            // Spawn task to listen for events, update UI, and auto-scroll
            spawn(async move {
                while let Some(ev) = rx.recv().await {
                    match &ev {
                        AgentEvent::Done => {
                            // Flush interview agent buffer if active.
                            if *interview_active.peek() {
                                let buf = interview_agent_buf.peek().clone();
                                if !buf.trim().is_empty() {
                                    interview_turns.write().push(InterviewTurn {
                                        is_agent: true,
                                        content: buf,
                                    });
                                }
                                interview_agent_buf.set(String::new());
                                interview_done.set(true);
                                interview_active.set(false);
                            }
                            is_working.set(false);
                            awaiting_feedback.set(None);
                            clear_stop_request();
                            continue;
                        }
                        AgentEvent::AwaitingFeedback(wf) => {
                            // Flush interview agent buffer as a dialog turn.
                            if *wf == Workflow::Interview {
                                let buf = interview_agent_buf.peek().clone();
                                if !buf.trim().is_empty() {
                                    interview_turns.write().push(InterviewTurn {
                                        is_agent: true,
                                        content: buf,
                                    });
                                }
                                interview_agent_buf.set(String::new());
                            }
                            is_working.set(false);
                            awaiting_feedback.set(Some(*wf));
                            feedback_text.set(String::new());
                            clear_stop_request();
                            continue;
                        }
                        AgentEvent::TrackerUpdate(pending) => {
                            issues.set(pending.clone());
                            continue;
                        }
                        _ => {}
                    }
                    // Accumulate agent text into the interview buffer.
                    if *interview_active.peek() {
                        if let AgentEvent::Claude(ClaudeEvent::Assistant { ref message }) = ev {
                            for block in &message.content {
                                if let ContentBlock::Text { text } = block {
                                    let mut buf = interview_agent_buf.write();
                                    if !buf.is_empty() {
                                        buf.push('\n');
                                    }
                                    buf.push_str(text);
                                }
                            }
                        }
                    }
                    // Extract file changes from tool use events
                    if let AgentEvent::Claude(ClaudeEvent::Assistant { ref message }) = ev {
                        for block in &message.content {
                            if let ContentBlock::ToolUse { name, input, .. } = block {
                                let (path, kind) = match name.as_str() {
                                    "Read" => (
                                        input.get("file_path").and_then(|v| v.as_str()),
                                        FileChangeKind::Read,
                                    ),
                                    "Write" => (
                                        input.get("file_path").and_then(|v| v.as_str()),
                                        FileChangeKind::Created,
                                    ),
                                    "Edit" => (
                                        input.get("file_path").and_then(|v| v.as_str()),
                                        FileChangeKind::Modified,
                                    ),
                                    _ => (None, FileChangeKind::Read),
                                };
                                if let Some(p) = path {
                                    let mut files = changed_files.write();
                                    // Update existing entry or add new
                                    if let Some(existing) = files.iter_mut().find(|f| f.path == p) {
                                        // Upgrade: Read -> Modified/Created
                                        if kind != FileChangeKind::Read {
                                            existing.kind = kind;
                                        }
                                    } else {
                                        files.push(ChangedFile {
                                            path: p.to_string(),
                                            kind,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    events.write().push(ev);
                    if *follow_mode.peek()
                        && let Some(el) = bottom_el.peek().as_ref()
                    {
                        let _ = el.scroll_to(ScrollBehavior::Instant).await;
                    }
                }
            });
        }
    });

    let refresh_tracker = move |_: MouseEvent| {
        info!("Refreshing trackers...");
        let infos = find_tracker();
        tracker_ids.set(infos.clone());
        let mut all_pending = Vec::new();
        for info in &infos {
            let body = get_tracker_body(info.number);
            all_pending.extend(parse_pending(&body));
        }
        all_pending.sort_by_key(|i| i.number);
        all_pending.dedup_by_key(|i| i.number);
        let mut prs = list_open_prs();
        // Phase 4 (#146): one batched GraphQL query populates the unresolved
        // thread counts for every open PR in a single round-trip, so the
        // sidebar's per-PR `(N)` badge stays in sync with the rest of the
        // refresh without N extra HTTP calls.
        let counts = fetch_unresolved_thread_counts(DEFAULT_REVIEW_BOT_LOGIN);
        for pr in &mut prs {
            pr.unresolved_thread_count = counts.get(&pr.number).copied().unwrap_or(0);
        }
        let pr_map = open_pr_map_from(&prs);
        pr_map_sig.set(pr_map);
        pull_requests.set(prs);
        issues.set(all_pending);
    };

    let start_work = move |tracker_num: u32| {
        clear_stop_request();
        is_working.set(true);
        changed_files.write().clear();
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_loop(&cfg, tracker_num);
        });
    };

    let start_single_issue = move |issue_num: u32| {
        clear_stop_request();
        is_working.set(true);
        changed_files.write().clear();
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_single_issue(&cfg, issue_num);
        });
    };

    let start_pr_fix = move |pr_num: u32| {
        clear_stop_request();
        is_working.set(true);
        changed_files.write().clear();
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_pr_review_fix(&cfg, pr_num);
        });
    };

    let on_start_workflow = move |workflow_id: String| {
        clear_stop_request();
        let cfg = config.read().clone();

        // Interview has special state management.
        if workflow_id == "interview" {
            is_working.set(true);
            interview_turns.write().clear();
            interview_agent_buf.set(String::new());
            interview_active.set(true);
            interview_done.set(false);
            tokio::spawn(async move {
                run_interview_draft(&cfg);
            });
            return;
        }

        // Security scan (local, non-agent).
        if workflow_id == "security_scan" {
            let root = cfg.root.clone();
            let targets = cfg.scan_targets.clone();
            info!("Running security review scan...");
            spawn(async move {
                let findings =
                    tokio::task::spawn_blocking(move || run_security_scan(&root, &targets))
                        .await
                        .unwrap_or_default();
                info!("Security scan complete: {} findings", findings.len());
                security_findings.set(findings);
            });
            return;
        }

        // Auto merge (special).
        if workflow_id == "auto_merge" {
            return;
        }

        is_working.set(true);

        // Look up a registered action runner, otherwise use the generic YAML runner.
        if let Some(action) = lookup_action(&workflow_id) {
            let action = *action;
            tokio::spawn(async move {
                let mut ctx = ActionContext::new(&workflow_id);
                if let Err(e) = action(&cfg, &mut ctx) {
                    agent::shell::log(&format!("Workflow '{}' failed: {e}", ctx.workflow_id));
                }
            });
        } else {
            tokio::spawn(async move { run_workflow_draft(&cfg, &workflow_id) });
        }
    };

    let save_settings = move |_: MouseEvent| {
        let cfg = config.read().clone();
        settings_status.set(Some("Saving configuration...".into()));
        spawn(async move {
            let root = cfg.root.clone();
            let result = tokio::task::spawn_blocking(move || {
                save_dev_config(&root, &cfg)?;

                match cfg.bot_settings.mode {
                    BotAuthMode::Token => {
                        let token = cfg.bot_settings.token.trim();
                        if token.is_empty() {
                            clear_bot_token(&root).map_err(|e| e.to_string())?;
                        } else {
                            store_bot_token(&root, token).map_err(|e| e.to_string())?;
                        }
                        clear_bot_private_key_pem(&root).map_err(|e| e.to_string())?;
                    }
                    BotAuthMode::GitHubApp => {
                        clear_bot_token(&root).map_err(|e| e.to_string())?;
                        let pem = cfg.bot_settings.private_key_pem.trim();
                        if pem.is_empty() {
                            clear_bot_private_key_pem(&root).map_err(|e| e.to_string())?;
                        } else {
                            store_bot_private_key_pem(&root, pem).map_err(|e| e.to_string())?;
                        }
                    }
                    BotAuthMode::Disabled => {
                        clear_bot_token(&root).map_err(|e| e.to_string())?;
                        clear_bot_private_key_pem(&root).map_err(|e| e.to_string())?;
                    }
                }

                let api_key = cfg.local_inference.api_key.trim();
                if api_key.is_empty() {
                    clear_local_inference_api_key(&root).map_err(|e| e.to_string())?;
                } else {
                    store_local_inference_api_key(&root, api_key).map_err(|e| e.to_string())?;
                }

                Ok::<(), String>(())
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);

            match result {
                Ok(()) => settings_status
                    .set(Some("Configuration saved. Secrets use the OS credential vault.".into())),
                Err(err) => {
                    settings_status.set(Some(format!("Failed to save configuration: {err}")))
                }
            }
        });
    };

    let submit_feedback = move |_: MouseEvent| {
        let fb = feedback_text.read().clone();
        if fb.trim().is_empty() {
            return;
        }
        clear_stop_request();
        let wf = *awaiting_feedback.read();
        awaiting_feedback.set(None);
        is_working.set(true);

        // Record user answer as interview turn.
        if wf == Some(Workflow::Interview) {
            interview_turns.write().push(InterviewTurn {
                is_agent: false,
                content: fb.clone(),
            });
        }

        let cfg = config.read().clone();
        tokio::spawn(async move {
            match wf {
                Some(Workflow::Interview) => run_interview_respond(&cfg, &fb),
                Some(w) => {
                    use agent::shell::run_workflow_finalize;
                    run_workflow_finalize(&cfg, w.to_id(), &fb);
                }
                None => {}
            }
        });
    };

    let stop_work = move |_: MouseEvent| {
        request_stop();
        is_working.set(false);
    };

    let on_auto_merge = move |_: MouseEvent| {
        auto_merge_enabled.set(true); // Optimistic guard against double-click
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                if let Some(pr) = current_branch_pr() {
                    if is_auto_merge_enabled(pr.number) {
                        return true;
                    }
                    return enable_auto_merge(pr.number);
                }
                false
            })
            .await;
            match result {
                Ok(true) => auto_merge_enabled.set(true),
                Ok(false) => {
                    info!("Failed to enable auto-merge");
                    auto_merge_enabled.set(false);
                }
                Err(e) => {
                    info!("Auto-merge task failed: {e}");
                    auto_merge_enabled.set(false);
                }
            }
        });
    };

    let css = format!("{vars}\n{BASE_CSS}", vars = theme.read().to_css_vars());

    rsx! {
        style { "{css}" }

        div { class: "ide",
            // ── Title bar ──
            div { class: "titlebar",
                div { class: "titlebar-left",
                    span { class: "titlebar-icon", ">" }
                    span { class: "titlebar-name", "{config.read().project_name} Dev Agent" }
                }
                div { class: "titlebar-right",
                    select {
                        class: "titlebar-select",
                        value: "{theme.read().name}",
                        onchange: move |evt| {
                            if let Some(t) = Theme::by_name(&evt.value()) {
                                theme.set(t);
                            }
                        },
                        for t in Theme::all() {
                            option { value: "{t.name}", "{t.name}" }
                        }
                    }
                }
            }

            // ── Main body: sidebar + editor ──
            div { class: "ide-body",
                Sidebar {
                    config,
                    tracker_ids,
                    issues,
                    pull_requests,
                    pr_map: pr_map_sig,
                    is_working,
                    awaiting_feedback,
                    feedback_text,
                    auto_merge_enabled,
                    settings_status,
                    refresh_tracker,
                    start_work,
                    start_single_issue,
                    start_pr_fix,
                    workflow_entries,
                    on_start_workflow,
                    save_settings,
                    stop_work,
                    submit_feedback,
                    on_auto_merge,
                }

                Editor {
                    events,
                    changed_files,
                    security_findings,
                    interview_turns,
                    interview_active,
                    interview_done,
                    root: root_sig,
                    follow_mode,
                    expand_all,
                    bottom_el,
                }
            }

            // ── Status bar ──
            Statusbar {
                config,
                tracker_ids,
                issues,
                events,
                is_working,
                theme_name: theme.read().name.to_string(),
            }
        }
    }
}
