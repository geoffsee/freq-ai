use crate::agent::cmd::{die, log};
use crate::agent::issue::preflight;
use crate::agent::launch::log_resolved_agent_launch;
use crate::agent::process::stop_requested;
use crate::agent::run::run_agent;
use crate::agent::types::{AgentEvent, Config, EVENT_SENDER, Workflow};

/// Inject standard variables that all workflows may need.
fn inject_common_vars(cfg: &Config, vars: &mut serde_json::Value) {
    vars["project_name"] = serde_json::Value::String(cfg.project_name.clone());
    vars["dry_run"] = serde_json::Value::Bool(cfg.dry_run);
    vars["user_personas_skill_path"] =
        serde_json::Value::String(cfg.skill_paths.user_personas.clone());
}

/// Run the draft phase of any two-phase workflow loaded from YAML.
pub fn run_workflow_draft(cfg: &Config, workflow_id: &str) {
    use crate::agent::workflow::{
        fetch_extra_context, gather_context_as_json, load_and_render, load_workflows,
    };

    let workflows = load_workflows(&cfg.root, &cfg.workflow_preset);
    let wf = workflows.get(workflow_id).unwrap_or_else(|| {
        die(&format!("Unknown workflow: {workflow_id}"));
    });
    let phase_cfg = wf.phases.get("draft").unwrap_or_else(|| {
        die(&format!("No draft phase in workflow '{workflow_id}'"));
    });

    preflight(cfg);
    log(&phase_cfg.log_start);

    let mut vars = gather_context_as_json(cfg, &wf.context);
    inject_common_vars(cfg, &mut vars);
    fetch_extra_context(wf, &mut vars);

    let prompt = load_and_render(&cfg.root, &cfg.workflow_preset, wf, "draft", &vars)
        .unwrap_or_else(|e| die(&format!("Prompt render failed: {e}")));

    if cfg.dry_run {
        log_resolved_agent_launch(cfg, &[]);
        log(&format!("[dry-run] Would run {} draft", wf.name));
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::Done);
        }
        return;
    }

    run_agent(cfg, &prompt);
    if stop_requested() {
        log(&format!("Stop requested. {} draft cancelled.", wf.name));
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::Done);
        }
        return;
    }

    log(&phase_cfg.log_complete);
    if let Some(wf_enum) = Workflow::from_id(workflow_id) {
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::AwaitingFeedback(wf_enum));
        }
    } else if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.send(AgentEvent::Done);
    }
}

/// Run the finalize phase of any two-phase workflow loaded from YAML.
pub fn run_workflow_finalize(cfg: &Config, workflow_id: &str, feedback: &str) {
    use crate::agent::workflow::{
        fetch_extra_context, gather_context_as_json, load_and_render, load_workflows,
    };

    let workflows = load_workflows(&cfg.root, &cfg.workflow_preset);
    let wf = workflows.get(workflow_id).unwrap_or_else(|| {
        die(&format!("Unknown workflow: {workflow_id}"));
    });
    let phase_cfg = wf.phases.get("finalize").unwrap_or_else(|| {
        die(&format!("No finalize phase in workflow '{workflow_id}'"));
    });

    preflight(cfg);
    log(&phase_cfg.log_start);

    let mut vars = gather_context_as_json(cfg, &wf.context);
    inject_common_vars(cfg, &mut vars);
    fetch_extra_context(wf, &mut vars);
    vars["feedback"] = serde_json::Value::String(feedback.to_string());

    let prompt = load_and_render(&cfg.root, &cfg.workflow_preset, wf, "finalize", &vars)
        .unwrap_or_else(|e| die(&format!("Prompt render failed: {e}")));

    run_agent(cfg, &prompt);
    if stop_requested() {
        log(&format!(
            "Stop requested. {} finalization cancelled.",
            wf.name
        ));
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::Done);
        }
        return;
    }

    log(&phase_cfg.log_complete);
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.send(AgentEvent::Done);
    }
}

pub fn run_sprint_planning_draft(cfg: &Config) {
    run_workflow_draft(cfg, "sprint_planning");
}

pub fn run_sprint_planning_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "sprint_planning", feedback);
}

pub fn run_retrospective_draft(cfg: &Config) {
    run_workflow_draft(cfg, "retrospective");
}

pub fn run_retrospective_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "retrospective", feedback);
}

pub fn gather_strategic_context_base(
    cfg: &Config,
) -> (String, String, String, String, String, String) {
    let ctx = crate::agent::workflow::gather_context_as_json(cfg, "strategic");
    (
        ctx["open_issues"].as_str().unwrap_or("[]").to_string(),
        ctx["open_prs"].as_str().unwrap_or("[]").to_string(),
        ctx["recent_commits"].as_str().unwrap_or("[]").to_string(),
        ctx["active_review_threads"]
            .as_str()
            .unwrap_or("[]")
            .to_string(),
        ctx["snapshot"].as_str().unwrap_or("").to_string(),
        ctx["project_status"].as_str().unwrap_or("").to_string(),
    )
}
