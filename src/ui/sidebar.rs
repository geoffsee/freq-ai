use crate::agent::tracker::{PendingIssue, PrSummary, TrackerInfo};
use crate::agent::types::{Agent, Config, LocalInferencePreset, Workflow};
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::LazyLock;

/// The canonical issue-comment skill, embedded at compile time.
///
/// `.agents/skills/issue-tracking/SKILL.md` is the single source
/// of truth for the trigger list rendered below. The Dev UI reminder reads
/// from this file via `parse_skill_triggers` so the human-facing nudge cannot
/// drift from what agents see when they load the skill. See issue #127.
const ISSUE_TRACKING_SKILL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.agents/skills/issue-tracking/SKILL.md"
));

/// Heading that bounds the trigger bullets inside `SKILL.md`. If you rename
/// this heading in the skill file, update the constant here in the same edit.
const ISSUE_COMMENT_TRIGGERS_HEADING: &str = "## Comment When One of These Triggers Fires";

static ISSUE_COMMENT_TRIGGERS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    parse_skill_triggers(ISSUE_TRACKING_SKILL).expect(
        "issue-tracking SKILL.md must define a non-empty trigger list under \
         the canonical heading; update the heading constant or restore the bullets",
    )
});

/// Extract the bulleted trigger list that lives under
/// `ISSUE_COMMENT_TRIGGERS_HEADING` in `SKILL.md`. Returns `None` if the
/// heading is missing or the section has no bullets, so initialization fails
/// freqly rather than rendering an empty reminder.
fn parse_skill_triggers(skill_md: &'static str) -> Option<Vec<&'static str>> {
    let heading_idx = skill_md.find(ISSUE_COMMENT_TRIGGERS_HEADING)?;
    let after_heading = &skill_md[heading_idx + ISSUE_COMMENT_TRIGGERS_HEADING.len()..];
    // Bound the section at the next top-level heading so we don't bleed into
    // sibling sections like "## Anti-patterns".
    let section = after_heading
        .find("\n## ")
        .map(|i| &after_heading[..i])
        .unwrap_or(after_heading);
    let triggers: Vec<&'static str> = section
        .lines()
        .filter_map(|line| line.strip_prefix("- "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
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
    rsx! {
        details { class: "issue-comment-details",
            summary { class: "issue-comment-summary", title: "GitHub issue comment reminder", "⚠️" }
            div { class: "issue-comment-reminder",
                div { class: "issue-comment-reminder-title", "Before marking an issue complete" }
                p { class: "issue-comment-reminder-copy",
                    "Write a GitHub issue comment only when it changes what the next reader would do."
                }
                ul { class: "issue-comment-reminder-list",
                    for trigger in ISSUE_COMMENT_TRIGGERS.iter() {
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
    bot_configured: Signal<bool>,
    refresh_tracker: EventHandler<MouseEvent>,
    start_work: EventHandler<u32>,
    start_single_issue: EventHandler<u32>,
    start_pr_fix: EventHandler<u32>,
    start_sprint_planning: EventHandler<MouseEvent>,
    start_code_review: EventHandler<MouseEvent>,
    start_strategic_review: EventHandler<MouseEvent>,
    start_roadmapper: EventHandler<MouseEvent>,
    start_retrospective: EventHandler<MouseEvent>,
    start_ideation: EventHandler<MouseEvent>,
    start_report: EventHandler<MouseEvent>,
    start_security_review: EventHandler<MouseEvent>,
    start_security_code_review: EventHandler<MouseEvent>,
    start_housekeeping: EventHandler<MouseEvent>,
    start_refresh_agents: EventHandler<MouseEvent>,
    start_refresh_docs: EventHandler<MouseEvent>,
    stop_work: EventHandler<MouseEvent>,
    submit_feedback: EventHandler<MouseEvent>,
    on_auto_merge: EventHandler<MouseEvent>,
) -> Element {
    let working = *is_working.read();
    let awaiting = *awaiting_feedback.read();
    let auto_merge = *auto_merge_enabled.read();
    let has_bot = *bot_configured.read();
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
                                    config.write().agent = agent;
                                }
                            },
                            option { value: "claude", "Claude" }
                            option { value: "codex", "Codex" }
                            option { value: "copilot", "Copilot" }
                            option { value: "gemini", "Gemini" }
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
                    }
                }
            }

            // Actions section
            div { class: "sidebar-section",
                div { class: "section-header", "ACTIONS" }
                div { class: "sidebar-buttons sidebar-buttons-col",
                    button {
                        class: "btn btn-sm btn-ideation",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_ideation.call(evt),
                        if working { "Working..." } else { "Ideation" }
                    }
                    button {
                        class: "btn btn-sm btn-report",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_report.call(evt),
                        if working { "Working..." } else { "UXR Synth" }
                    }
                    button {
                        class: "btn btn-sm btn-strategy",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_strategic_review.call(evt),
                        if working { "Working..." } else { "Strategic Review" }
                    }
                    button {
                        class: "btn btn-sm btn-strategy",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_roadmapper.call(evt),
                        if working { "Working..." } else { "Roadmapper" }
                    }
                    button {
                        class: "btn btn-sm btn-action",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_sprint_planning.call(evt),
                        if working { "Working..." } else { "Sprint Planning" }
                    }
                    hr { class: "sidebar-buttons-divider" }
                    button {
                        class: "btn btn-sm btn-action",
                        disabled: working || awaiting.is_some() || !has_bot,
                        title: if !has_bot { "Bot credentials required. Set DEV_BOT_TOKEN env var or configure a GitHub App (see README)." } else { "" },
                        onclick: move |evt| start_code_review.call(evt),
                        if !has_bot { "Code Review (no bot)" } else if working { "Working..." } else { "Code Review" }
                    }
                    button {
                        class: "btn btn-sm btn-security",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_security_review.call(evt),
                        if working { "Working..." } else { "Security Review" }
                    }
                    button {
                        class: "btn btn-sm btn-retro",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_retrospective.call(evt),
                        if working { "Working..." } else { "Retrospective" }
                    }
                    button {
                        class: "btn btn-sm btn-security",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_security_code_review.call(evt),
                        if working { "Working..." } else { "Security Code Review" }
                    }
                    hr { class: "sidebar-buttons-divider" }
                    button {
                        class: "btn btn-sm btn-action",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_housekeeping.call(evt),
                        if working { "Working..." } else { "Housekeeping" }
                    }
                    button {
                        class: "btn btn-sm btn-action",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_refresh_agents.call(evt),
                        if working { "Working..." } else { "Refresh Agents" }
                    }
                    button {
                        class: "btn btn-sm btn-action",
                        disabled: working || awaiting.is_some(),
                        onclick: move |evt| start_refresh_docs.call(evt),
                        if working { "Working..." } else { "Refresh Docs" }
                    }
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

    /// Guards issue #127: the Dev UI reminder must read its trigger list
    /// directly from the canonical SKILL.md so the two cannot drift.
    #[test]
    fn skill_md_yields_nonempty_trigger_list() {
        let parsed = parse_skill_triggers(ISSUE_TRACKING_SKILL)
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
        // Sanity check: the LazyLock initializer must agree with a fresh parse.
        assert_eq!(&*ISSUE_COMMENT_TRIGGERS, &parsed);
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
}
