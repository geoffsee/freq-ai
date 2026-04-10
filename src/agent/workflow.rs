use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::agent::shell::{cmd_stdout, log};
use crate::agent::tracker::list_open_prs;
use crate::agent::types::Config;

// ── YAML config types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub pattern: ExecutionPattern,
    #[serde(default = "default_context")]
    pub context: String,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Named action from the registry. When set, the generic dispatch calls
    /// this action instead of `run_workflow_draft`.
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default)]
    pub extra_context: Vec<ExtraContextFetch>,
    #[serde(default)]
    pub phases: IndexMap<String, PhaseConfig>,
    #[serde(default)]
    pub fragments: HashMap<String, String>,
}

/// Fetch the body of a GitHub issue by its label and inject it as a template variable.
#[derive(Debug, Deserialize)]
pub struct ExtraContextFetch {
    /// Template variable name to inject (e.g. "report_synthesis").
    pub name: String,
    /// GitHub issue label to search for (e.g. "uxr-synthesis").
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPattern {
    TwoPhase,
    OneShot,
    MultiRound,
    Implementation,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_order")]
    pub order: u32,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub requires_bot: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            category: default_category(),
            order: default_order(),
            visible: true,
            requires_bot: false,
        }
    }
}

/// Lightweight summary of a workflow for the UI sidebar.
#[derive(Clone, Debug)]
pub struct WorkflowEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub order: u32,
    pub requires_bot: bool,
}

fn category_rank(category: &str) -> (u8, &str) {
    match category {
        "discovery" => (0, category),
        "planning" => (1, category),
        "review" => (2, category),
        "maintenance" => (3, category),
        _ => (4, category),
    }
}

/// Load all workflow configs and return sorted sidebar entries for a preset.
pub fn load_sidebar_entries(root: &str, preset: &str) -> Vec<WorkflowEntry> {
    let workflows = load_workflows(root, preset);
    let mut entries: Vec<WorkflowEntry> = workflows
        .values()
        .filter(|wf| wf.ui.visible)
        .map(|wf| WorkflowEntry {
            id: wf.id.clone(),
            name: wf.name.clone(),
            category: wf.ui.category.clone(),
            order: wf.ui.order,
            requires_bot: wf.ui.requires_bot,
        })
        .collect();
    entries.sort_by(|a, b| {
        category_rank(&a.category)
            .cmp(&category_rank(&b.category))
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
}

#[derive(Debug, Deserialize)]
pub struct PhaseConfig {
    pub template: String,
    #[serde(default)]
    pub log_start: String,
    #[serde(default)]
    pub log_complete: String,
}

fn default_context() -> String {
    "none".to_string()
}
fn default_category() -> String {
    "other".to_string()
}
fn default_order() -> u32 {
    99
}
fn default_true() -> bool {
    true
}

// ── Loader ───────────────────────────────────────────────────────────────

/// List available preset names by scanning subdirectories of `.agents/workflows/`.
pub fn list_presets(root: &str) -> Vec<String> {
    let base = Path::new(root).join("assets/workflows");
    let mut presets = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path
                    .join(".")
                    .read_dir()
                    .is_ok_and(|mut d| d.next().is_some())
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                presets.push(name.to_string());
            }
        }
    }
    presets.sort();
    // Ensure "default" comes first if present.
    if let Some(pos) = presets.iter().position(|p| p == "default") {
        presets.remove(pos);
        presets.insert(0, "default".to_string());
    }
    presets
}

/// Scan `.agents/workflows/{preset}/*/workflow.yaml` and return a map
/// keyed by workflow `id`.
pub fn load_workflows(root: &str, preset: &str) -> HashMap<String, WorkflowConfig> {
    let mut map = HashMap::new();
    let base = Path::new(root).join("assets/workflows").join(preset);
    let entries = match std::fs::read_dir(&base) {
        Ok(e) => e,
        Err(_) => return map,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let yaml_path = path.join("workflow.yaml");
        let content = match std::fs::read_to_string(&yaml_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        match serde_yaml::from_str::<WorkflowConfig>(&content) {
            Ok(wf) => {
                map.insert(wf.id.clone(), wf);
            }
            Err(e) => {
                log(&format!(
                    "WARNING: failed to parse {}: {e}",
                    yaml_path.display()
                ));
            }
        }
    }
    map
}

/// Read a prompt template file from a workflow directory within a preset.
pub fn load_template(root: &str, preset: &str, workflow_dir: &str, filename: &str) -> String {
    let path = Path::new(root)
        .join("assets/workflows")
        .join(preset)
        .join(workflow_dir)
        .join(filename);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        log(&format!(
            "WARNING: failed to read template {}: {e}",
            path.display()
        ));
        String::new()
    })
}

// ── Template rendering ───────────────────────────────────────────────────

/// Render a Handlebars template with the given variables and fragment partials.
pub fn render_prompt(
    template: &str,
    vars: &serde_json::Value,
    fragments: &HashMap<String, String>,
) -> Result<String, String> {
    let mut hbs = handlebars::Handlebars::new();
    hbs.set_strict_mode(false); // missing vars render as empty string

    for (name, body) in fragments {
        hbs.register_partial(name, body)
            .map_err(|e| format!("Fragment '{name}' parse error: {e}"))?;
    }

    hbs.render_template(template, vars)
        .map_err(|e| format!("Template render error: {e}"))
}

/// Convenience: load a workflow's phase template and render it.
pub fn load_and_render(
    root: &str,
    preset: &str,
    wf: &WorkflowConfig,
    phase: &str,
    vars: &serde_json::Value,
) -> Result<String, String> {
    let phase_cfg = wf
        .phases
        .get(phase)
        .ok_or_else(|| format!("No phase '{phase}' in workflow '{}'", wf.id))?;

    // Derive the directory name from the workflow id (underscore → hyphen)
    let dir = wf.id.replace('_', "-");
    let template = load_template(root, preset, &dir, &phase_cfg.template);
    if template.is_empty() {
        return Err(format!(
            "Empty template '{}' for workflow '{}'",
            phase_cfg.template, wf.id
        ));
    }
    render_prompt(&template, vars, &wf.fragments)
}

// ── Context gathering (JSON wrappers) ────────────────────────────────────

/// Gather context for the given gatherer name, returning a JSON object.
pub fn gather_context_as_json(cfg: &Config, gatherer: &str) -> serde_json::Value {
    match gatherer {
        "sprint" => {
            let open_issues = gh_open_issues(50);
            let open_prs = open_prs_json();
            let status = read_project_file(&cfg.root, "STATUS.md");
            let issues_md = read_project_file(&cfg.root, "ISSUES.md");
            serde_json::json!({
                "open_issues": open_issues,
                "open_prs": open_prs,
                "status": status,
                "issues_md": issues_md,
            })
        }
        "strategic" => {
            let open_issues = gh_open_issues(50);
            let open_prs = open_prs_json();
            let recent_commits = cmd_stdout("git", &["log", "--oneline", "--no-decorate", "-30"])
                .unwrap_or_default();
            let crate_tree =
                cmd_stdout("ls", &["-1", &format!("{}/crates", cfg.root)]).unwrap_or_default();
            let status = read_project_file(&cfg.root, "STATUS.md");
            let issues_md = read_project_file(&cfg.root, "ISSUES.md");
            serde_json::json!({
                "open_issues": open_issues,
                "open_prs": open_prs,
                "recent_commits": recent_commits,
                "crate_tree": crate_tree,
                "status": status,
                "issues_md": issues_md,
            })
        }
        "retro" => {
            let recent_commits = cmd_stdout("git", &["log", "--oneline", "--no-decorate", "-50"])
                .unwrap_or_default();
            let closed_issues = cmd_stdout(
                "gh",
                &[
                    "issue",
                    "list",
                    "--state",
                    "closed",
                    "--json",
                    "number,title,closedAt",
                    "--limit",
                    "30",
                ],
            )
            .unwrap_or_else(|| "[]".to_string());
            let merged_prs = cmd_stdout(
                "gh",
                &[
                    "pr",
                    "list",
                    "--state",
                    "merged",
                    "--json",
                    "number,title,mergedAt",
                    "--limit",
                    "30",
                ],
            )
            .unwrap_or_else(|| "[]".to_string());
            let open_issues = gh_open_issues(50);
            let open_prs = open_prs_json();
            let status = read_project_file(&cfg.root, "STATUS.md");
            let issues_md = read_project_file(&cfg.root, "ISSUES.md");
            serde_json::json!({
                "recent_commits": recent_commits,
                "closed_issues": closed_issues,
                "merged_prs": merged_prs,
                "open_issues": open_issues,
                "open_prs": open_prs,
                "status": status,
                "issues_md": issues_md,
            })
        }
        "housekeeping" => {
            let open_issues = cmd_stdout(
                "gh",
                &[
                    "issue",
                    "list",
                    "--state",
                    "open",
                    "--json",
                    "number,title,labels,updatedAt,assignees",
                    "--limit",
                    "100",
                ],
            )
            .unwrap_or_else(|| "[]".to_string());
            let open_prs = open_prs_json();
            let local_branches =
                cmd_stdout("git", &["branch", "--format=%(refname:short)"]).unwrap_or_default();
            let trackers = crate::agent::tracker::find_tracker();
            let mut tracker_bodies = String::new();
            for t in &trackers {
                let body = crate::agent::tracker::get_tracker_body(t.number);
                tracker_bodies.push_str(&format!(
                    "### Tracker #{} — {}\n{}\n\n",
                    t.number, t.title, body
                ));
            }
            let status = read_project_file(&cfg.root, "STATUS.md");
            let issues_md = read_project_file(&cfg.root, "ISSUES.md");
            serde_json::json!({
                "open_issues": open_issues,
                "open_prs": open_prs,
                "local_branches": local_branches,
                "tracker_bodies": tracker_bodies,
                "status": status,
                "issues_md": issues_md,
            })
        }
        _ => serde_json::json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_entries_group_by_category_then_order() {
        let root = env!("CARGO_MANIFEST_DIR");
        let entries = load_sidebar_entries(root, "default");
        let labels: Vec<(&str, &str)> = entries
            .iter()
            .map(|entry| (entry.category.as_str(), entry.name.as_str()))
            .collect();

        assert_eq!(
            labels,
            vec![
                ("discovery", "Ideation"),
                ("discovery", "UXR Synth"),
                ("discovery", "Interview"),
                ("planning", "Strategic Review"),
                ("planning", "Roadmapper"),
                ("planning", "Sprint Planning"),
                ("review", "Code Review"),
                ("review", "Security Review"),
                ("review", "Security Code Review"),
                ("review", "Retrospective"),
                ("maintenance", "Housekeeping"),
                ("maintenance", "Refresh Agents"),
                ("maintenance", "Refresh Docs"),
                ("maintenance", "Auto Merge"),
            ]
        );
    }
}

/// Fetch extra context variables declared in `extra_context` and inject them
/// into the vars map.
pub fn fetch_extra_context(wf: &WorkflowConfig, vars: &mut serde_json::Value) {
    for fetch in &wf.extra_context {
        let val = fetch_issue_by_label(&fetch.label);
        vars[&fetch.name] = serde_json::Value::String(val);
    }
}

/// Fetch the body of the most recent open GitHub issue with the given label.
/// Returns `"# <title>\n\n<body>"` or empty string if none found.
fn fetch_issue_by_label(label: &str) -> String {
    cmd_stdout(
        "gh",
        &[
            "issue",
            "list",
            "--label",
            label,
            "--state",
            "open",
            "--limit",
            "1",
            "--json",
            "number,title,body",
            "--jq",
            ".[0] // empty | \"# \\(.title)\\n\\n\\(.body)\"",
        ],
    )
    .unwrap_or_default()
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn gh_open_issues(limit: u32) -> String {
    cmd_stdout(
        "gh",
        &[
            "issue",
            "list",
            "--state",
            "open",
            "--json",
            "number,title,labels",
            "--limit",
            &limit.to_string(),
        ],
    )
    .unwrap_or_else(|| "[]".to_string())
}

fn open_prs_json() -> String {
    let prs = list_open_prs();
    serde_json::to_string_pretty(&prs).unwrap_or_else(|_| "[]".to_string())
}

fn read_project_file(root: &str, name: &str) -> String {
    std::fs::read_to_string(format!("{root}/{name}")).unwrap_or_default()
}
