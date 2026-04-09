use crate::agent::types::{
    AgentEvent, ChangedFile, ClaudeEvent, ContentBlock, FileChangeKind, InterviewTurn,
};
use crate::ui::components::EventRow;
use crate::ui::security::{SecurityFinding, SecurityPanel};
use dioxus::prelude::*;
use std::collections::HashMap;

/// Build a map of tool_use_id -> tool_name from all events so far.
fn build_tool_names(events: &[AgentEvent]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for ev in events {
        if let AgentEvent::Claude(ClaudeEvent::Assistant { message }) = ev {
            for block in &message.content {
                if let ContentBlock::ToolUse { id, name, .. } = block {
                    map.insert(id.clone(), name.clone());
                }
            }
        }
    }
    map
}

#[derive(Clone, Copy, PartialEq)]
enum EditorTab {
    Output,
    Files,
    Security,
    Interview,
}

#[component]
pub fn Editor(
    events: Signal<Vec<AgentEvent>>,
    changed_files: Signal<Vec<ChangedFile>>,
    security_findings: Signal<Vec<SecurityFinding>>,
    interview_turns: Signal<Vec<InterviewTurn>>,
    interview_active: Signal<bool>,
    interview_done: Signal<bool>,
    root: Signal<String>,
    follow_mode: Signal<bool>,
    expand_all: Signal<bool>,
    bottom_el: Signal<Option<std::rc::Rc<MountedData>>>,
) -> Element {
    let mut active_tab = use_signal(|| EditorTab::Output);
    let tool_names = build_tool_names(&events.read());

    let files = changed_files.read();
    let created_count = files
        .iter()
        .filter(|f| f.kind == FileChangeKind::Created)
        .count();
    let modified_count = files
        .iter()
        .filter(|f| f.kind == FileChangeKind::Modified)
        .count();
    let read_count = files
        .iter()
        .filter(|f| f.kind == FileChangeKind::Read)
        .count();
    let file_count = files.len();
    drop(files);

    rsx! {
        div { class: "editor",
            // Tab bar
            div { class: "tab-bar",
                div {
                    class: if *active_tab.read() == EditorTab::Output { "tab tab-active" } else { "tab" },
                    onclick: move |_| active_tab.set(EditorTab::Output),
                    "Agent Output"
                }
                div {
                    class: if *active_tab.read() == EditorTab::Files { "tab tab-active" } else { "tab" },
                    onclick: move |_| active_tab.set(EditorTab::Files),
                    "Files ({file_count})"
                }
                div {
                    class: if *active_tab.read() == EditorTab::Security { "tab tab-active" } else { "tab" },
                    onclick: move |_| active_tab.set(EditorTab::Security),
                    "Security ({security_findings.read().len()})"
                }
                if *interview_active.read() || !interview_turns.read().is_empty() {
                    div {
                        class: if *active_tab.read() == EditorTab::Interview { "tab tab-active" } else { "tab" },
                        onclick: move |_| active_tab.set(EditorTab::Interview),
                        "Interview ({interview_turns.read().len()})"
                    }
                }
                div { class: "tab-actions",
                    if *active_tab.read() == EditorTab::Output {
                        label { class: "tab-check",
                            input {
                                r#type: "checkbox",
                                checked: *follow_mode.read(),
                                onchange: move |evt| follow_mode.set(evt.value().parse::<bool>().unwrap_or(false)),
                            }
                            span { "Follow" }
                        }
                        label { class: "tab-check",
                            input {
                                r#type: "checkbox",
                                checked: *expand_all.read(),
                                onchange: move |evt| expand_all.set(evt.value().parse::<bool>().unwrap_or(false)),
                            }
                            span { "Expand All" }
                        }
                    }
                }
            }

            match *active_tab.read() {
                EditorTab::Output => rsx! {
                    div { class: "editor-content",
                        for (i , event) in events.read().iter().enumerate() {
                            EventRow {
                                key: "{i}",
                                event: event.clone(),
                                expand_all: *expand_all.read(),
                                tool_names: tool_names.clone(),
                            }
                        }
                        if events.read().is_empty() {
                            div { class: "text-muted editor-empty", "Waiting for activity..." }
                        }
                        div {
                            onmounted: move |cx| bottom_el.set(Some(cx.data())),
                        }
                    }
                },
                EditorTab::Files => rsx! {
                    div { class: "editor-content",
                        if file_count == 0 {
                            div { class: "text-muted editor-empty", "No file activity yet..." }
                        } else {
                            div { class: "files-summary",
                                if created_count > 0 {
                                    span { class: "file-stat file-stat-created", "+{created_count} created" }
                                }
                                if modified_count > 0 {
                                    span { class: "file-stat file-stat-modified", "~{modified_count} modified" }
                                }
                                if read_count > 0 {
                                    span { class: "file-stat file-stat-read", "{read_count} read" }
                                }
                            }
                            ul { class: "file-list",
                                for (i , file) in changed_files.read().iter().enumerate() {
                                    li { key: "{i}", class: "file-entry",
                                        span {
                                            class: match file.kind {
                                                FileChangeKind::Created => "file-kind file-kind-created",
                                                FileChangeKind::Modified => "file-kind file-kind-modified",
                                                FileChangeKind::Deleted => "file-kind file-kind-deleted",
                                                FileChangeKind::Read => "file-kind file-kind-read",
                                            },
                                            match file.kind {
                                                FileChangeKind::Created => "+",
                                                FileChangeKind::Modified => "~",
                                                FileChangeKind::Deleted => "-",
                                                FileChangeKind::Read => " ",
                                            }
                                        }
                                        span { class: "file-path", "{file.path}" }
                                    }
                                }
                            }
                        }
                    }
                },
                EditorTab::Security => rsx! {
                    SecurityPanel {
                        findings: security_findings,
                        root,
                    }
                },
                EditorTab::Interview => rsx! {
                    div { class: "interview-panel",
                        if interview_turns.read().is_empty() {
                            div { class: "interview-empty", "Interview not started yet." }
                        }
                        for (i , turn) in interview_turns.read().iter().enumerate() {
                            if turn.is_agent {
                                div { key: "{i}", class: "interview-turn interview-turn-agent",
                                    div { class: "interview-role interview-role-agent", "Agent" }
                                    div { class: "interview-bubble interview-bubble-agent",
                                        "{turn.content}"
                                    }
                                }
                            } else {
                                div { key: "{i}", class: "interview-turn interview-turn-user",
                                    div { class: "interview-role interview-role-user", "You" }
                                    div { class: "interview-bubble interview-bubble-user",
                                        "{turn.content}"
                                    }
                                }
                            }
                        }
                        if *interview_done.read() && !interview_turns.read().is_empty() {
                            div { class: "interview-summary-card",
                                div { class: "interview-summary-title", "Interview Complete" }
                                div { class: "interview-summary-body",
                                    "The structured summary has been generated above. You can use it as input for other workflows (Sprint Planning, Roadmapper, etc.)."
                                }
                            }
                        }
                        if *interview_active.read() && !*interview_done.read() {
                            div { class: "interview-status",
                                "Awaiting your response in the Feedback section..."
                            }
                        }
                    }
                },
            }
        }
    }
}
