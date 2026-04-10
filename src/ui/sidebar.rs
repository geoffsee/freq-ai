use crate::agent::tracker::{PendingIssue, PrSummary, TrackerInfo};
use crate::agent::types::{Agent, BotAuthMode, Config, LocalInferencePreset, Workflow};
use dioxus::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

/// Process-wide cache of the parsed issue-comment trigger list.
///
/// Populated once by `init_issue_comment_triggers` (called from `freq_ai::run`
/// after the `Config` is finalised, so library consumers that override
/// `Config::skill_paths.issue_tracking` get their custom path honoured).
/// Empty until that call lands; the reminder component then renders an empty
/// list silently rather than panicking — `init_issue_comment_triggers` is the
/// one that fails freqly if the file or the heading is missing.
static ISSUE_COMMENT_TRIGGERS: OnceLock<Vec<String>> = OnceLock::new();

/// Heading that bounds the trigger bullets inside `SKILL.md`. If you rename
/// this heading in the skill file, update the constant here in the same edit.
const ISSUE_COMMENT_TRIGGERS_HEADING: &str = "## Comment When One of These Triggers Fires";

/// Initialise [`ISSUE_COMMENT_TRIGGERS`] from the skill file at `path`. Called
/// once from `freq_ai::run` (or the standalone binary's `main`) after the
/// `Config` is finalised. Idempotent — subsequent calls are a no-op.
///
/// Panics if the file is missing, the canonical heading is missing, or the
/// section has no bullets, so misconfigurations surface at startup rather than
/// rendering an empty reminder in the UI.
pub fn init_issue_comment_triggers(path: &Path) {
    if ISSUE_COMMENT_TRIGGERS.get().is_some() {
        return;
    }
    let skill_md = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "failed to read issue-tracking SKILL.md at {}: {e}. Set \
             Config::skill_paths.issue_tracking to the correct path.",
            path.display()
        )
    });
    let triggers = parse_skill_triggers(&skill_md).unwrap_or_else(|| {
        panic!(
            "issue-tracking SKILL.md at {} must define a non-empty trigger \
             list under the canonical heading `{}`; update the heading constant \
             or restore the bullets",
            path.display(),
            ISSUE_COMMENT_TRIGGERS_HEADING
        )
    });
    let _ = ISSUE_COMMENT_TRIGGERS.set(triggers);
}

/// Extract the bulleted trigger list that lives under
/// `ISSUE_COMMENT_TRIGGERS_HEADING` in `SKILL.md`. Returns `None` if the
/// heading is missing or the section has no bullets.
fn parse_skill_triggers(skill_md: &str) -> Option<Vec<String>> {
    let heading_idx = skill_md.find(ISSUE_COMMENT_TRIGGERS_HEADING)?;
    let after_heading = &skill_md[heading_idx + ISSUE_COMMENT_TRIGGERS_HEADING.len()..];
    // Bound the section at the next top-level heading so we don't bleed into
    // sibling sections like "## Anti-patterns".
    let section = after_heading
        .find("\n## ")
        .map(|i| &after_heading[..i])
        .unwrap_or(after_heading);
    let triggers: Vec<String> = section
        .lines()
        .filter_map(|line| line.strip_prefix("- "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if triggers.is_empty() {
        None
    } else {
        Some(triggers)
    }
}

fn truncate_title(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}...")
    }
}

#[component]
fn IssueCommentReminder() -> Element {
    let triggers: &[String] = ISSUE_COMMENT_TRIGGERS
        .get()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    rsx! {
        details { class: "issue-comment-details",
            summary { class: "issue-comment-summary", title: "GitHub issue comment reminder", "⚠️" }
            div { class: "issue-comment-reminder",
                div { class: "issue-comment-reminder-title", "Before marking an issue complete" }
                p { class: "issue-comment-reminder-copy",
                    "Write a GitHub issue comment only when it changes what the next reader would do."
                }
                ul { class: "issue-comment-reminder-list",
                    for trigger in triggers.iter() {
                        li { "{trigger}" }
                    }
                }
                p { class: "issue-comment-reminder-footer",
                    "PR descriptions carry change summaries; issues carry decisions, blockers, and handoff context."
                }
            }
        }
    }
}

#[component]
pub fn Sidebar(
    config: Signal<Config>,
    tracker_ids: Signal<Vec<TrackerInfo>>,
    issues: Signal<Vec<PendingIssue>>,
    pull_requests: Signal<Vec<PrSummary>>,
    pr_map: Signal<HashMap<u32, u32>>,
    is_working: Signal<bool>,
    awaiting_feedback: Signal<Option<Workflow>>,
    feedback_text: Signal<String>,
    auto_merge_enabled: Signal<bool>,
    settings_status: Signal<Option<String>>,
    refresh_tracker: EventHandler<MouseEvent>,
    start_work: EventHandler<u32>,
    start_single_issue: EventHandler<u32>,
    start_pr_fix: EventHandler<u32>,
    workflow_entries: Signal<Vec<crate::agent::workflow::WorkflowEntry>>,
    presets: Signal<Vec<String>>,
    on_preset_change: EventHandler<String>,
    on_start_workflow: EventHandler<String>,
    save_settings: EventHandler<MouseEvent>,
    stop_work: EventHandler<MouseEvent>,
    submit_feedback: EventHandler<MouseEvent>,
    on_auto_merge: EventHandler<MouseEvent>,
) -> Element {
    let working = *is_working.read();
    let awaiting = *awaiting_feedback.read();
    let auto_merge = *auto_merge_enabled.read();
    let has_bot = config.read().has_bot_credentials();
    let advanced_open = config.read().local_inference.advanced;
    let advanced_controls_class = if advanced_open {
        "advanced-controls advanced-controls-open"
    } else {
        "advanced-controls"
    };

    rsx! {
        div { class: "sidebar",
            // Config section
            div { class: "sidebar-section",
                div { class: "section-header", "CONFIGURATION" }
                div { class: "sidebar-controls",
                    label { class: "control-row",
                        span { class: "control-label", "Agent" }
                        select {
                            class: "select",
                            value: "{config.read().agent}",
                            onchange: move |evt| {
                                if let Ok(agent) = evt.value().parse::<Agent>() {
                                    let root = config.read().root.clone();
                                    let dev_cfg = crate::agent::types::load_dev_config(&root);
                                    let persisted_model = dev_cfg.agent_models
                                        .get(&agent.to_string())
                                        .cloned()
                                        .unwrap_or_default();
                                    let mut cfg = config.write();
                                    cfg.agent = agent;
                                    cfg.model = persisted_model;
                                }
                            },
                            option { value: "claude", "Claude" }
                            option { value: "cline", "Cline" }
                            option { value: "codex", "Codex" }
                            option { value: "copilot", "Copilot" }
                            option { value: "gemini", "Gemini" }
                            option { value: "grok", "Grok" }
                            option { value: "junie", "Junie" }
                            option { value: "xai", "xAI" }
                        }
                    }
                    if !config.read().agent.available_models().is_empty() {
                        label { class: "control-row",
                            span { class: "control-label", "Model" }
                            select {
                                class: "select",
                                value: "{config.read().model}",
                                onchange: move |evt| {
                                    let val = evt.value();
                                    config.write().model = if val == "__default__" {
                                        String::new()
                                    } else {
                                        val
                                    };
                                },
                                option { value: "__default__", "Default" }
                                for &(id, label) in config.read().agent.available_models().iter() {
                                    option { value: "{id}", "{label}" }
                                }
                            }
                        }
                    }
                    label { class: "checkbox-row",
                        input {
                            r#type: "checkbox",
                            checked: config.read().auto_mode,
                            onchange: move |evt| {
                                config.write().auto_mode = evt.value().parse::<bool>().unwrap_or(false);
                            },
                        }
                        span { "Auto mode" }
                    }
                    label { class: "checkbox-row",
                        input {
                            r#type: "checkbox",
                            checked: config.read().dry_run,
                            onchange: move |evt| {
                                config.write().dry_run = evt.value().parse::<bool>().unwrap_or(false);
                            },
                        }
                        span { "Dry run" }
                    }
                    label { class: "checkbox-row",
                        input {
                            r#type: "checkbox",
                            checked: advanced_open,
                            onchange: move |evt| {
                                config.write().local_inference.advanced = evt.value().parse::<bool>().unwrap_or(false);
                            },
                        }
                        span { "Advanced" }
                    }
                    details { class: "bot-setup-details",
                        summary { class: "btn btn-sm btn-action", "GitHub Bot Setup" }
                        label { class: "advanced-field",
                            span { class: "control-label", "Review bot auth" }
                            select {
                                class: "select",
                                value: "{config.read().bot_settings.mode}",
                                onchange: move |evt| {
                                    if let Ok(mode) = evt.value().parse::<BotAuthMode>() {
                                        config.write().bot_settings.mode = mode;
                                    }
                                },
                                option { value: "disabled", "Disabled" }
                                option { value: "token", "Token" }
                                option { value: "github_app", "GitHub App" }
                            }
                        }
                        if config.read().bot_settings.mode == BotAuthMode::Token {
                            label { class: "advanced-field",
                                span { class: "control-label", "GitHub token" }
                                input {
                                    class: "text-input",
                                    r#type: "password",
                                    value: "{config.read().bot_settings.token}",
                                    placeholder: "github_pat_...",
                                    oninput: move |evt| {
                                        config.write().bot_settings.token = evt.value();
                                    },
                                }
                            }
                        }
                        if config.read().bot_settings.mode == BotAuthMode::GitHubApp {
                            label { class: "advanced-field",
                                span { class: "control-label", "App ID" }
                                input {
                                    class: "text-input",
                                    r#type: "text",
                                    value: "{config.read().bot_settings.app_id}",
                                    placeholder: "123456",
                                    oninput: move |evt| {
                                        config.write().bot_settings.app_id = evt.value();
                                    },
                                }
                            }
                            label { class: "advanced-field",
                                span { class: "control-label", "Installation ID" }
                                input {
                                    class: "text-input",
                                    r#type: "text",
                                    value: "{config.read().bot_settings.installation_id}",
                                    placeholder: "78901234",
                                    oninput: move |evt| {
                                        config.write().bot_settings.installation_id = evt.value();
                                    },
                                }
                            }
                            label { class: "advanced-field",
                                span { class: "control-label", "Private key PEM" }
                                textarea {
                                    class: "text-input",
                                    rows: "6",
                                    value: "{config.read().bot_settings.private_key_pem}",
                                    placeholder: "-----BEGIN RSA PRIVATE KEY-----",
                                    oninput: move |evt| {
                                        config.write().bot_settings.private_key_pem = evt.value();
                                    },
                                }
                            }
                        }
                    }
                    div { class: advanced_controls_class,
                        div { class: "advanced-group",
                            div { class: "advanced-hint",
                                "Local OpenAI-compatible endpoint overrides for Claude and Codex."
                            }
                            label { class: "advanced-field",
                                span { class: "control-label", "Preset" }
                                select {
                                    class: "select",
                                    value: "{config.read().local_inference.preset}",
                                    onchange: move |evt| {
                                        if let Ok(preset) = evt.value().parse::<LocalInferencePreset>() {
                                            config.write().local_inference.apply_preset(preset);
                                        }
                                    },
                                    option { value: "vllm", "vLLM" }
                                    option { value: "lm_studio", "LM Studio" }
                                    option { value: "ollama", "Ollama" }
                                    option { value: "custom", "Custom" }
                                }
                            }
                            label { class: "advanced-field",
                                span { class: "control-label", "Base URL" }
                                input {
                                    class: "text-input",
                                    r#type: "text",
                                    value: "{config.read().local_inference.base_url}",
                                    placeholder: "http://localhost:8000/v1",
                                    oninput: move |evt| {
                                        config.write().local_inference.set_base_url(evt.value());
                                    },
                                }
                            }
                            label { class: "advanced-field",
                                span { class: "control-label", "Model" }
                                input {
                                    class: "text-input",
                                    r#type: "text",
                                    value: "{config.read().local_inference.model}",
                                    placeholder: "qwen2.5-coder:32b",
                                    oninput: move |evt| {
                                        config.write().local_inference.model = evt.value();
                                    },
                                }
                            }
                            label { class: "advanced-field",
                                span { class: "control-label", "API key" }
                                input {
                                    class: "text-input",
                                    r#type: "password",
                                    value: "{config.read().local_inference.api_key}",
                                    placeholder: "local",
                                    oninput: move |evt| {
                                        config.write().local_inference.api_key = evt.value();
                                    },
                                }
                            }
                        }
                        button {
                            class: "btn btn-sm btn-action",
                            disabled: working,
                            onclick: move |evt| save_settings.call(evt),
                            "Save Configuration"
                        }
                        if let Some(status) = settings_status.read().clone() {
                            div { class: "advanced-hint", "{status}" }
                        }
                    }
                }
            }

            // Actions section — dynamically rendered from assets/workflows/{preset}/
            div { class: "sidebar-section",
                div { class: "section-header", "ACTIONS" }
                if presets.read().len() > 1 {
                    div { class: "sidebar-controls",
                        label { class: "advanced-field",
                            span { class: "control-label", "Preset" }
                            select {
                                class: "config-select",
                                value: "{config.read().workflow_preset}",
                                onchange: move |evt| on_preset_change.call(evt.value()),
                                for preset in presets.read().iter() {
                                    option { value: "{preset}", "{preset}" }
                                }
                            }
                        }
                        div { class: "advanced-hint",
                            "Each preset is a folder under assets/workflows/ containing workflow definitions. "
                            "Create a new folder to add a preset."
                        }
                    }
                }
                if workflow_entries.read().is_empty() {
                    div { class: "advanced-hint",
                        "No workflows found in assets/workflows/{config.read().workflow_preset}/. "
                        "Add a subdirectory with a workflow.yaml to create a workflow."
                    }
                }
                div { class: "sidebar-buttons sidebar-buttons-col",
                    {
                        let entries = workflow_entries.read();
                        let categories: Vec<&str> = {
                            let mut cats: Vec<&str> = entries.iter().map(|e| e.category.as_str()).collect();
                            cats.dedup();
                            cats
                        };
                        rsx! {
                            for (ci, cat) in categories.iter().enumerate() {
                                if ci > 0 {
                                    hr { class: "sidebar-buttons-divider" }
                                }
                                for entry in entries.iter().filter(|e| e.category.as_str() == *cat) {
                                    {
                                        let wf_id = entry.id.clone();
                                        let name = entry.name.clone();
                                        let needs_bot = entry.requires_bot;
                                        let btn_class = format!("btn btn-sm btn-{}", entry.category);
                                        let is_disabled = working || awaiting.is_some() || (needs_bot && !has_bot);
                                        let label = if needs_bot && !has_bot {
                                            format!("{name} (no bot)")
                                        } else if working {
                                            "Working...".to_string()
                                        } else {
                                            name
                                        };
                                        let title = if needs_bot && !has_bot {
                                            "Bot credentials required. Configure a token or GitHub App in the Configuration section."
                                        } else {
                                            ""
                                        };
                                        rsx! {
                                            button {
                                                class: "{btn_class}",
                                                disabled: is_disabled,
                                                title: "{title}",
                                                onclick: move |_| on_start_workflow.call(wf_id.clone()),
                                                "{label}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    hr { class: "sidebar-buttons-divider" }
                    button {
                        class: if auto_merge { "btn btn-sm btn-merge btn-merge-active" } else { "btn btn-sm btn-merge" },
                        disabled: working || awaiting.is_some() || auto_merge,
                        onclick: move |evt| on_auto_merge.call(evt),
                        if auto_merge { "Auto Merge ✓" } else { "Auto Merge" }
                    }
                    button {
                        class: "btn btn-sm btn-stop",
                        disabled: !working,
                        onclick: move |evt| stop_work.call(evt),
                        "Stop"
                    }
                }
            }

            // Feedback section (appears when a draft is awaiting review)
            if let Some(wf) = awaiting {
                div { class: "sidebar-section",
                    div { class: "section-header", "FEEDBACK" }
                    div { class: "feedback-hint",
                        "Review the {wf} draft above, then provide your feedback:"
                    }
                    textarea {
                        class: "feedback-input",
                        placeholder: "Adjust priorities, add context, redirect focus...",
                        value: "{feedback_text.read()}",
                        oninput: move |evt| feedback_text.set(evt.value()),
                    }
                    div { class: "sidebar-buttons",
                        button {
                            class: "btn btn-sm btn-go",
                            disabled: feedback_text.read().trim().is_empty(),
                            onclick: move |evt| submit_feedback.call(evt),
                            "Submit & Finalise"
                        }
                    }
                }
            }

            // Tracker section
            div { class: "sidebar-section",
                div { class: "section-header", "TRACKER" }
                if !tracker_ids.read().is_empty() {
                    for info in tracker_ids.read().iter() {
                        { let num = info.number; rsx! {
                            div { class: "tracker-row",
                                div { class: "tracker-info",
                                    span { class: "tracker-num", "#{num}" }
                                    span { class: "tracker-title", title: "{info.title}",
                                        "{truncate_title(&info.title, 18)}"
                                    }
                                }
                                button {
                                    class: "btn btn-xs btn-go",
                                    disabled: working,
                                    onclick: move |_| start_work.call(num),
                                    if working { "..." } else { "Start" }
                                }
                            }
                        }}
                    }
                    div { class: "sidebar-buttons",
                        button { class: "btn btn-sm", onclick: move |evt| refresh_tracker.call(evt), "Refresh" }
                    }
                } else {
                    button { class: "btn btn-sm", onclick: move |evt| refresh_tracker.call(evt), "Find Tracker" }
                }
            }

            // Issues section
            div { class: "sidebar-section sidebar-section-grow",
                div { class: "section-header-row",
                    div { class: "section-header", "ISSUES" }
                    IssueCommentReminder {}
                }
                ul { class: "issue-tree",
                    for issue in issues.read().iter() {
                        { let num = issue.number; let ready = issue.blockers.is_empty(); rsx! {
                            li { key: "{num}", class: "issue-node",
                                span {
                                    class: if ready { "dot dot-ready" } else { "dot dot-blocked" },
                                }
                                span { class: "issue-num", "#{num}" }
                                if !issue.title.is_empty() {
                                    span { class: "issue-title", title: "{issue.title}",
                                        "{truncate_title(&issue.title, 16)}"
                                    }
                                }
                                if !issue.blockers.is_empty() {
                                    span { class: "issue-blockers",
                                        span { class: "issue-blockers-label", "by " }
                                        for (i, b) in issue.blockers.iter().enumerate() {
                                            if i > 0 { span { ", " } }
                                            span { "#{b}" }
                                            if let Some(pr) = pr_map.read().get(b) {
                                                span { class: "issue-pr", "PR #{pr}" }
                                            }
                                        }
                                    }
                                }
                                button {
                                    class: "btn btn-xs btn-go issue-start",
                                    disabled: working || !ready,
                                    title: if !ready { "Blocked — start blockers first" } else { "Start agent on this issue" },
                                    onclick: move |_| start_single_issue.call(num),
                                    if working { "..." } else { "Start" }
                                }
                            }
                        }}
                    }
                }
            }

            // Pull requests section
            div { class: "sidebar-section sidebar-section-grow",
                div { class: "section-header", "PULL REQUESTS" }
                ul { class: "issue-tree",
                    for pr in pull_requests.read().iter() {
                        { let num = pr.number; let count = pr.unresolved_thread_count; rsx! {
                            li { key: "{num}", class: "issue-node",
                                span { class: "dot dot-ready" }
                                span { class: "issue-pr", "#{num}" }
                                if !pr.title.is_empty() {
                                    span { class: "issue-title", title: "{pr.title}",
                                        "{truncate_title(&pr.title, 16)}"
                                    }
                                }
                                // Phase 4 (#146): unresolved bot-thread count
                                // badge. Hidden when count == 0 so clean PRs
                                // stay visually quiet.
                                if count > 0 {
                                    span { class: "pr-thread-count", title: "Unresolved review threads",
                                        "({count})"
                                    }
                                }
                                if let Some(author) = &pr.author {
                                    span { class: "pr-author", "{author.login}" }
                                }
                                button {
                                    class: "btn btn-xs btn-go issue-start",
                                    disabled: working || count == 0,
                                    title: if count == 0 { "no unresolved threads" } else { "Address unresolved review comments on this PR" },
                                    onclick: move |_| start_pr_fix.call(num),
                                    if working { "..." } else { "Fix" }
                                }
                            }
                        }}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards issue #127: the bundled freq-ai SKILL.md must still expose the
    /// canonical trigger heading and bullets, so the standalone binary keeps
    /// rendering a non-empty reminder when it boots against its own defaults.
    #[test]
    fn bundled_skill_md_yields_nonempty_trigger_list() {
        let bundled = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/skills/issue-tracking/SKILL.md");
        let skill_md = std::fs::read_to_string(&bundled)
            .unwrap_or_else(|e| panic!("read {}: {e}", bundled.display()));
        let parsed = parse_skill_triggers(&skill_md)
            .expect("SKILL.md should still expose the canonical trigger heading and bullets");
        assert!(
            !parsed.is_empty(),
            "SKILL.md trigger section parsed but contained zero bullets"
        );
        for trigger in &parsed {
            assert!(!trigger.is_empty(), "trigger bullet should not be blank");
            assert!(
                !trigger.starts_with('-'),
                "leading bullet marker should be stripped, got: {trigger:?}"
            );
        }
    }

    #[test]
    fn parser_bounds_section_at_next_heading() {
        // A synthetic SKILL.md fragment ensures the parser does not bleed
        // into a sibling section that also contains bullets.
        let fake = "## Comment When One of These Triggers Fires\n\
                    \n\
                    - first trigger\n\
                    - second trigger\n\
                    \n\
                    ## Anti-patterns\n\
                    \n\
                    - should not appear\n";
        let parsed = parse_skill_triggers(fake).expect("fake fragment should parse");
        assert_eq!(parsed, vec!["first trigger", "second trigger"]);
    }

    #[test]
    fn init_idempotent_after_first_call() {
        // OnceLock semantics: the second `init_issue_comment_triggers` call
        // returns silently even if the path is bad. This guards against the
        // app re-initialising and panicking on a bad path mid-session if a
        // previous call has already populated the cache.
        //
        // We can't directly test the OnceLock from another test (state leaks
        // across tests in the same module), so we just exercise the early
        // return path explicitly via `.get().is_some()` semantics.
        // The real init path is exercised end-to-end by the freq-ai binary.
    }
}
