use crate::agent::tracker::TrackerInfo;
use crate::agent::types::Config;
use dioxus::prelude::*;

#[component]
pub fn Statusbar(
    config: Signal<Config>,
    tracker_ids: Signal<Vec<TrackerInfo>>,
    issues: Signal<Vec<crate::agent::tracker::PendingIssue>>,
    events: Signal<Vec<crate::agent::types::AgentEvent>>,
    is_working: Signal<bool>,
    theme_name: String,
) -> Element {
    let working = *is_working.read();
    let issue_count = issues.read().len();
    let event_count = events.read().len();
    let tracker_nums = tracker_ids
        .read()
        .iter()
        .map(|t| format!("#{}", t.number))
        .collect::<Vec<_>>()
        .join(", ");

    rsx! {
        div { class: "statusbar",
            div { class: "statusbar-left",
                span { class: if working { "status-dot status-dot-active" } else { "status-dot status-dot-idle" } }
                span { if working { "Working" } else { "Ready" } }
                if !tracker_nums.is_empty() {
                    span { class: "status-sep", "|" }
                    span { "{tracker_nums}" }
                }
            }
            div { class: "statusbar-right",
                span { "{issue_count} issues" }
                span { class: "status-sep", "|" }
                span { "{event_count} events" }
                span { class: "status-sep", "|" }
                span { "{config.read().agent}" }
                span { class: "status-sep", "|" }
                span { "{theme_name}" }
            }
        }
    }
}
