use crate::agent::bot::resolve_bot_token;
use crate::agent::cmd::{cmd_run, cmd_stdout, log};
use crate::agent::issue::preflight;
use crate::agent::launch::log_resolved_agent_launch;
use crate::agent::process::{emit_event, stop_requested};
use crate::agent::run::run_agent_with_env;
use crate::agent::tracker::{
    build_code_review_prompt, build_security_review_prompt, list_open_prs, pr_body, pr_diff,
};
use crate::agent::types::{AgentEvent, Config};
use std::path::PathBuf;

pub fn run_code_review(cfg: &Config) {
    preflight(cfg);
    log("Starting code review...");

    // Resolve bot token so the review subprocess runs under the bot identity.
    let bot_token = cfg
        .effective_bot_credentials()
        .as_ref()
        .and_then(resolve_bot_token);

    if bot_token.is_none() {
        log(
            "WARNING: No bot credentials configured — reviews will run under your identity \
             (same-author approvals will fail). Set DEV_BOT_TOKEN or configure a GitHub App.",
        );
    }

    let extra_env: Vec<(String, String)> = bot_token
        .as_deref()
        .map(|t| vec![("GH_TOKEN".to_string(), t.to_string())])
        .unwrap_or_default();

    let prs = list_open_prs();
    if prs.is_empty() {
        log("No open PRs to review.");
        emit_event(AgentEvent::Done);
        return;
    }

    log(&format!("Found {} open PR(s)", prs.len()));
    if cfg.dry_run {
        log_resolved_agent_launch(cfg, &[]);
    }

    for pr in &prs {
        log(&format!("Reviewing PR #{}: {}", pr.number, pr.title));

        if cfg.dry_run {
            log(&format!("[dry-run] Would review PR #{}", pr.number));
            continue;
        }

        let body = pr_body(pr.number);
        let diff = pr_diff(pr.number);
        let prompt =
            build_code_review_prompt(&cfg.project_name, pr.number, &pr.title, &body, &diff);
        run_agent_with_env(cfg, &prompt, &extra_env);
        if stop_requested() {
            log("Stop requested. Code review cancelled.");
            emit_event(AgentEvent::Done);
            return;
        }

        log(&format!("Completed review of PR #{}", pr.number));
    }

    log("All code reviews complete.");
    emit_event(AgentEvent::Done);
}

pub fn run_security_code_review(cfg: &Config) {
    use crate::agent::snapshot::generate_codebase_snapshot;
    preflight(cfg);
    log("Starting security code review...");

    let crate_tree = cmd_stdout("tree", &["-L", "2", "crates"]).unwrap_or_default();
    let snapshot = generate_codebase_snapshot(&cfg.root);
    let prompt =
        build_security_review_prompt(&cfg.project_name, &crate_tree, &snapshot, cfg.dry_run);
    run_agent_with_env(cfg, &prompt, &[]);
    emit_event(AgentEvent::Done);
}

/// RAII guard that removes a git worktree on drop, including on panic.
pub struct WorktreeGuard {
    pub path: PathBuf,
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let path_str = self.path.to_string_lossy().to_string();
        if !cmd_run("git", &["worktree", "remove", "--force", &path_str]) {
            log(&format!(
                "WARNING: `git worktree remove` failed for {path_str}; falling back to fs cleanup"
            ));
            let _ = std::fs::remove_dir_all(&self.path);
            let _ = cmd_run("git", &["worktree", "prune"]);
        }
    }
}

pub fn run_pr_review_fix(cfg: &Config, pr_num: u32) {
    preflight(cfg);
    log(&format!("Starting Fix Comments run for PR #{pr_num}..."));

    if cfg.dry_run {
        log(&format!(
            "[dry-run] Would run Fix Comments for PR #{pr_num}"
        ));
        emit_event(AgentEvent::Done);
        return;
    }

    // Implementation continues...
    log("Fix Comments logic not fully implemented in this step.");
    emit_event(AgentEvent::Done);
}
