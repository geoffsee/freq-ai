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

use agent::shell::{
    clear_stop_request, parse_args, preflight, request_stop, run_code_review,
    run_housekeeping_draft, run_housekeeping_finalize, run_ideation_draft, run_ideation_finalize,
    run_loop, run_pr_review_fix, run_refresh_agents, run_refresh_docs, run_report_draft,
    run_report_finalize, run_retrospective_draft, run_retrospective_finalize, run_roadmapper_draft,
    run_roadmapper_finalize, run_security_code_review, run_single_issue, run_sprint_planning_draft,
    run_sprint_planning_finalize, run_strategic_review_draft, run_strategic_review_finalize,
};
use agent::tracker::{
    DEFAULT_REVIEW_BOT_LOGIN, PendingIssue, PrSummary, TrackerInfo, current_branch_pr,
    enable_auto_merge, fetch_unresolved_thread_counts, find_tracker, get_tracker_body,
    is_auto_merge_enabled, list_open_prs, open_pr_map_from, parse_pending,
};
use agent::types::{
    AgentEvent, ChangedFile, ClaudeEvent, ContentBlock, EVENT_SENDER, FileChangeKind, Workflow,
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
        Some(Commands::Ideation) => run_ideation_draft(&config),
        Some(Commands::UxrSynth) => run_report_draft(&config),
        Some(Commands::StrategicReview) => run_strategic_review_draft(&config),
        Some(Commands::Roadmapper) => run_roadmapper_draft(&config),
        Some(Commands::SprintPlanning) => run_sprint_planning_draft(&config),
        Some(Commands::Retrospective) => run_retrospective_draft(&config),
        Some(Commands::Housekeeping) => run_housekeeping_draft(&config),
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
    let root_sig = use_signal(|| config.read().root.clone());
    let mut auto_merge_enabled = use_signal(|| false);
    let bot_configured = use_signal(|| config.read().bot_credentials.is_some());
    let expand_all = use_signal(|| false);
    let follow_mode = use_signal(|| true);
    let bottom_el = use_signal(|| None::<std::rc::Rc<MountedData>>);
    let mut theme = use_signal(Theme::tokyo_night);

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
                            is_working.set(false);
                            awaiting_feedback.set(None);
                            clear_stop_request();
                            continue;
                        }
                        AgentEvent::AwaitingFeedback(wf) => {
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

    let start_sprint_planning = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_sprint_planning_draft(&cfg);
        });
    };

    let start_code_review = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_code_review(&cfg);
        });
    };

    let start_strategic_review = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_strategic_review_draft(&cfg);
        });
    };

    let start_roadmapper = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_roadmapper_draft(&cfg);
        });
    };

    let start_retrospective = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_retrospective_draft(&cfg);
        });
    };

    let start_ideation = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_ideation_draft(&cfg);
        });
    };

    let start_report = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_report_draft(&cfg);
        });
    };

    let start_security_review = move |_: MouseEvent| {
        let root = config.read().root.clone();
        let targets = config.read().scan_targets.clone();
        info!("Running security review scan...");
        spawn(async move {
            let findings = tokio::task::spawn_blocking(move || run_security_scan(&root, &targets))
                .await
                .unwrap_or_default();
            info!("Security scan complete: {} findings", findings.len());
            security_findings.set(findings);
        });
    };

    let start_security_code_review = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_security_code_review(&cfg);
        });
    };

    let start_housekeeping = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_housekeeping_draft(&cfg);
        });
    };

    let start_refresh_agents = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_refresh_agents(&cfg);
        });
    };

    let start_refresh_docs = move |_: MouseEvent| {
        clear_stop_request();
        is_working.set(true);
        let cfg = config.read().clone();
        tokio::spawn(async move {
            run_refresh_docs(&cfg);
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
        let cfg = config.read().clone();
        tokio::spawn(async move {
            match wf {
                Some(Workflow::Ideation) => run_ideation_finalize(&cfg, &fb),
                Some(Workflow::ReportResearch) => run_report_finalize(&cfg, &fb),
                Some(Workflow::StrategicReview) => run_strategic_review_finalize(&cfg, &fb),
                Some(Workflow::SprintPlanning) => run_sprint_planning_finalize(&cfg, &fb),
                Some(Workflow::Retrospective) => run_retrospective_finalize(&cfg, &fb),
                Some(Workflow::Roadmapper) => run_roadmapper_finalize(&cfg, &fb),
                Some(Workflow::Housekeeping) => run_housekeeping_finalize(&cfg, &fb),
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
                    bot_configured,
                    refresh_tracker,
                    start_work,
                    start_single_issue,
                    start_pr_fix,
                    start_sprint_planning,
                    start_code_review,
                    start_strategic_review,
                    start_roadmapper,
                    start_retrospective,
                    start_ideation,
                    start_report,
                    start_security_review,
                    start_security_code_review,
                    start_housekeeping,
                    start_refresh_agents,
                    start_refresh_docs,
                    stop_work,
                    submit_feedback,
                    on_auto_merge,
                }

                Editor {
                    events,
                    changed_files,
                    security_findings,
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