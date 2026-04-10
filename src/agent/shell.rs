use crate::agent::assets::{AGENTS_MD, SkillAssets};
use crate::agent::config_store::{
    load_bot_private_key_pem, load_bot_token, load_local_inference_api_key,
};
use crate::agent::tracker::{
    DEFAULT_REVIEW_BOT_LOGIN, build_code_review_prompt, build_interview_draft_prompt,
    build_interview_followup_prompt, build_interview_summary_prompt, build_lint_fix_prompt,
    build_pr_review_fix_prompt, build_prompt, build_refresh_agents_prompt,
    build_refresh_docs_prompt, build_security_review_prompt, build_test_fix_prompt,
    check_off_issue, close_issue, fetch_issue, fetch_unresolved_review_threads,
    find_retro_issues, find_upstream_branch, get_tracker_body, is_ready,
    list_open_prs, parse_completed, parse_pending, pr_body, pr_diff, pr_head_branch,
    resolve_review_thread,
};
use crate::agent::types::Workflow;
use crate::agent::types::{
    Agent, AgentEvent, AssistantMessage, BRANCH_PREFIX, BotCredentials, BotSettings,
    ClaudeEvent, Config, ContentBlock, EVENT_SENDER, MAX_COMMIT_ATTEMPTS, MAX_PUSH_ATTEMPTS,
};
use std::collections::BTreeSet;
use std::env;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use toak_rs::{MarkdownGenerator, MarkdownGeneratorOptions, count_tokens};
use tracing::info;

/// Maximum tokens to include from the codebase snapshot in a prompt.
const MAX_SNAPSHOT_TOKENS: usize = 100_000;
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static ACTIVE_CHILD_PID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
static BOT_TOKEN_CACHE: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();

/// Interview round tracking. Answers accumulate across rounds so the follow-up
/// and summary prompts can reference prior responses.
static INTERVIEW_ANSWERS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Cached bot tokens expire after 50 minutes (GitHub installation tokens last 60).
const TOKEN_CACHE_SECS: u64 = 50 * 60;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AgentLaunchOverrides {
    args: Vec<String>,
    env: Vec<(String, String)>,
}

/// Log the elapsed time for a labelled operation.
macro_rules! timed {
    ($label:expr, $body:expr) => {{
        let _t0 = Instant::now();
        let _result = $body;
        log(&format!("[timing] {} completed in {:.2?}", $label, _t0.elapsed()));
        _result
    }};
}

pub fn die(msg: &str) -> ! {
    eprintln!("ERROR: {msg}");
    process::exit(1);
}

pub fn log(msg: &str) {
    info!("{msg}");
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.send(AgentEvent::Log(msg.to_string()));
    }
}

// ── Action wrappers for the registry ─────────────────────────────────────

pub fn action_code_review(
    cfg: &Config,
    _ctx: &mut crate::agent::actions::ActionContext,
) -> Result<(), String> {
    run_code_review(cfg);
    Ok(())
}

pub fn action_security_code_review(
    cfg: &Config,
    _ctx: &mut crate::agent::actions::ActionContext,
) -> Result<(), String> {
    run_security_code_review(cfg);
    Ok(())
}

pub fn action_refresh_agents(
    cfg: &Config,
    _ctx: &mut crate::agent::actions::ActionContext,
) -> Result<(), String> {
    run_refresh_agents(cfg);
    Ok(())
}

pub fn action_refresh_docs(
    cfg: &Config,
    _ctx: &mut crate::agent::actions::ActionContext,
) -> Result<(), String> {
    run_refresh_docs(cfg);
    Ok(())
}

/// Run a command, return trimmed stdout or None on failure.
pub fn cmd_stdout(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Run a command, return trimmed stdout or die.
pub fn cmd_stdout_or_die(program: &str, args: &[&str], context: &str) -> String {
    cmd_stdout(program, args).unwrap_or_else(|| die(context))
}

/// Run a command, inheriting stdio. Returns success bool.
pub fn cmd_run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a command, capture combined stdout+stderr. Returns (success, output).
pub fn cmd_capture(program: &str, args: &[&str]) -> (bool, String) {
    match Command::new(program)
        .args(args)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
    {
        Ok(o) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            (o.status.success(), combined)
        }
        Err(e) => (false, e.to_string()),
    }
}

pub fn has_command(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ── Bot credentials ──

fn bot_token_cache() -> &'static Mutex<Option<(String, Instant)>> {
    BOT_TOKEN_CACHE.get_or_init(|| Mutex::new(None))
}

/// Load bot credentials from environment variables.
///
/// Resolution order:
/// 1. `DEV_BOT_TOKEN` — direct token (PAT or pre-minted installation token)
/// 2. `DEV_BOT_TOKEN_PATH` — path to a file containing the token
/// 3. `DEV_BOT_APP_ID` + `DEV_BOT_INSTALLATION_ID` + `DEV_BOT_PRIVATE_KEY` — GitHub App
pub fn load_bot_credentials_from_env() -> Option<BotCredentials> {
    // Direct token from env
    if let Ok(token) = env::var("DEV_BOT_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(BotCredentials::Token(token));
        }
    }

    // Token from file
    if let Ok(path) = env::var("DEV_BOT_TOKEN_PATH")
        && let Ok(token) = std::fs::read_to_string(&path)
    {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(BotCredentials::Token(token));
        }
    }

    // GitHub App credentials
    let app_id = env::var("DEV_BOT_APP_ID").ok().filter(|s| !s.is_empty())?;
    let installation_id = env::var("DEV_BOT_INSTALLATION_ID")
        .ok()
        .filter(|s| !s.is_empty())?;
    let private_key_path = env::var("DEV_BOT_PRIVATE_KEY").unwrap_or_else(|_| {
        env::var("HOME")
            .map(|h| format!("{h}/.config/freq-cloud/dev-ui-bot.pem"))
            .unwrap_or_else(|_| ".config/freq-cloud/dev-ui-bot.pem".to_string())
    });
    let private_key_pem = std::fs::read_to_string(&private_key_path)
        .map_err(|e| {
            log(&format!(
                "Failed to read bot private key at {private_key_path}: {e}"
            ))
        })
        .ok()?;

    Some(BotCredentials::GitHubApp {
        app_id,
        installation_id,
        private_key_pem,
    })
}

fn load_bot_settings(root: &str, dev_cfg: &crate::agent::types::DevConfig) -> BotSettings {
    if let Some(creds) = load_bot_credentials_from_env() {
        return BotSettings::from_credentials(&creds);
    }

    let mut settings = dev_cfg.bot.clone().into_bot_settings();
    if let Some(token) = load_bot_token(root) {
        settings.token = token;
    }
    if let Some(private_key_pem) = load_bot_private_key_pem(root) {
        settings.private_key_pem = private_key_pem;
    }
    settings
}

/// Resolve bot credentials to a usable `GH_TOKEN` value.
pub fn resolve_bot_token(creds: &BotCredentials) -> Option<String> {
    match creds {
        BotCredentials::Token(t) => Some(t.clone()),
        BotCredentials::GitHubApp {
            app_id,
            installation_id,
            private_key_pem,
        } => {
            // Check cache
            if let Ok(cache) = bot_token_cache().lock()
                && let Some((ref token, ref created_at)) = *cache
                && created_at.elapsed() < std::time::Duration::from_secs(TOKEN_CACHE_SECS)
            {
                return Some(token.clone());
            }

            let token = mint_installation_token(app_id, installation_id, private_key_pem)?;

            if let Ok(mut cache) = bot_token_cache().lock() {
                *cache = Some((token.clone(), Instant::now()));
            }

            Some(token)
        }
    }
}

/// Mint a GitHub App installation token via JWT + REST API.
fn mint_installation_token(
    app_id: &str,
    installation_id: &str,
    private_key_pem: &str,
) -> Option<String> {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| log(&format!("Invalid RSA PEM key: {e}")))
        .ok()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    let claims = serde_json::json!({
        "iss": app_id,
        "iat": now.saturating_sub(60),
        "exp": now + 600,
    });

    let jwt = encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|e| log(&format!("JWT signing failed: {e}")))
        .ok()?;

    let url = format!("https://api.github.com/app/installations/{installation_id}/access_tokens");

    // Pass the JWT-bearing Authorization header to curl via stdin (`--config -`)
    // instead of as a `-H` argv value, so the short-lived JWT is never visible
    // to other local users via `ps aux` / `/proc/<pid>/cmdline`.
    let mut child = Command::new("curl")
        .args([
            "--config",
            "-",
            "-s",
            "-X",
            "POST",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
            &url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| log(&format!("curl spawn failed: {e}")))
        .ok()?;

    {
        use std::io::Write;
        let mut stdin = child.stdin.take()?;
        let auth_config = format!("header = \"Authorization: Bearer {jwt}\"\n");
        stdin
            .write_all(auth_config.as_bytes())
            .map_err(|e| log(&format!("curl stdin write failed: {e}")))
            .ok()?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| log(&format!("curl failed: {e}")))
        .ok()?;

    if !output.status.success() {
        log("Failed to mint bot installation token (curl error)");
        return None;
    }

    let body: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| log(&format!("Failed to parse installation token response: {e}")))
        .ok()?;

    let token = body.get("token").and_then(|t| t.as_str()).map(String::from);
    if token.is_none() {
        let msg = body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        log(&format!(
            "GitHub API error minting installation token: {msg}"
        ));
    }
    token
}

fn active_child_pid_slot() -> &'static Mutex<Option<u32>> {
    ACTIVE_CHILD_PID.get_or_init(|| Mutex::new(None))
}

fn set_active_child_pid(pid: Option<u32>) {
    if let Ok(mut slot) = active_child_pid_slot().lock() {
        *slot = pid;
    }
}

fn active_child_pid() -> Option<u32> {
    active_child_pid_slot().lock().ok().and_then(|slot| *slot)
}

pub fn clear_stop_request() {
    STOP_REQUESTED.store(false, Ordering::SeqCst);
}

pub fn stop_requested() -> bool {
    STOP_REQUESTED.load(Ordering::SeqCst)
}

pub fn request_stop() {
    STOP_REQUESTED.store(true, Ordering::SeqCst);
    if let Some(pid) = active_child_pid() {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}

fn emit_event(ev: AgentEvent) {
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.send(ev);
    }
}

fn local_inference_api_key(api_key: &str) -> String {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        "local".to_string()
    } else {
        trimmed.to_string()
    }
}

fn local_inference_overrides(cfg: &Config) -> AgentLaunchOverrides {
    let local = &cfg.local_inference;
    if !local.advanced {
        return AgentLaunchOverrides::default();
    }

    let base_url = local.base_url.trim();
    if base_url.is_empty() {
        return AgentLaunchOverrides::default();
    }

    let mut overrides = AgentLaunchOverrides::default();
    let model = local.model.trim();

    match cfg.agent {
        Agent::Claude => {
            overrides
                .env
                .push(("ANTHROPIC_BASE_URL".to_string(), base_url.to_string()));
            overrides.env.push((
                "ANTHROPIC_API_KEY".to_string(),
                local_inference_api_key(&local.api_key),
            ));
        }
        Agent::Codex => {
            overrides
                .env
                .push(("OPENAI_BASE_URL".to_string(), base_url.to_string()));
            overrides.env.push((
                "OPENAI_API_KEY".to_string(),
                local_inference_api_key(&local.api_key),
            ));
            // The `-c key=value` value portion is parsed as TOML by Codex,
            // so the URL must be a TOML string literal. Debug formatting
            // (`{base_url:?}`) emits the value wrapped in `"…"`, which is
            // exactly what TOML expects. Verified against Codex 0.118.0
            // (#142): both the quoted and unquoted forms resolve to the
            // same endpoint, but only the quoted form is correct under
            // the documented TOML grammar.
            overrides
                .args
                .extend(["-c".to_string(), format!("openai_base_url={base_url:?}")]);
        }
        Agent::Copilot | Agent::Gemini => return AgentLaunchOverrides::default(),
    }

    if !model.is_empty() {
        overrides
            .args
            .extend(["--model".to_string(), model.to_string()]);
    }

    overrides
}

fn merged_agent_env(cfg: &Config, extra_env: &[(String, String)]) -> Vec<(String, String)> {
    let mut env = local_inference_overrides(cfg).env;
    env.extend(extra_env.iter().cloned());
    env
}

fn redact_launch_env_value(key: &str, value: &str) -> String {
    if key.ends_with("API_KEY") && !value.is_empty() && value != "local" {
        "<redacted>".to_string()
    } else {
        value.to_string()
    }
}

fn log_resolved_agent_launch(cfg: &Config, extra_env: &[(String, String)]) {
    let overrides = local_inference_overrides(cfg);
    let env = merged_agent_env(cfg, extra_env);
    let args = if overrides.args.is_empty() {
        "(none)".to_string()
    } else {
        overrides.args.join(" ")
    };
    let env = if env.is_empty() {
        "(none)".to_string()
    } else {
        env.iter()
            .map(|(key, value)| format!("{key}={}", redact_launch_env_value(key, value)))
            .collect::<Vec<_>>()
            .join(", ")
    };

    log(&format!(
        "[dry-run] Agent launch overrides for {} -> args: {args}; env: {env}",
        cfg.agent
    ));
}

fn run_claude_native_with_env(
    binary: &str,
    args: &[String],
    extra_env: &[(String, String)],
    cwd: Option<&Path>,
) -> bool {
    let mut cmd = Command::new(binary);
    cmd.args(args)
        // Clear API keys so agents use the user's subscription instead of API credits.
        .env("ANTHROPIC_API_KEY", "")
        .env("OPENAI_API_KEY", "")
        .env("GEMINI_API_KEY", "");
    if let Some(p) = cwd {
        cmd.current_dir(p);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|_| panic!("failed to spawn {binary}"));
    set_active_child_pid(Some(child.id()));

    let stdout = child.stdout.take().expect("piped stdout");
    let reader = BufReader::new(stdout);

    for line in reader.lines().map_while(Result::ok) {
        if stop_requested() {
            let _ = child.kill();
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(ev) = serde_json::from_str::<ClaudeEvent>(trimmed) {
            emit_event(AgentEvent::Claude(ev));
        } else {
            log(&format!("claude: {trimmed}"));
        }
    }
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    set_active_child_pid(None);
    ok
}

fn u64_to_u32(value: Option<u64>) -> Option<u32> {
    value.and_then(|v| u32::try_from(v).ok())
}

fn assistant_text_event(text: String) -> AgentEvent {
    AgentEvent::Claude(ClaudeEvent::Assistant {
        message: AssistantMessage {
            content: vec![ContentBlock::Text { text }],
        },
    })
}

fn assistant_block_event(block: ContentBlock) -> AgentEvent {
    AgentEvent::Claude(ClaudeEvent::Assistant {
        message: AssistantMessage {
            content: vec![block],
        },
    })
}

fn codex_events_from_json_line(line: &str) -> Option<Vec<AgentEvent>> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = v.get("type")?.as_str()?;

    let mut out = Vec::new();
    match event_type {
        "thread.started" => {
            let description = v
                .get("thread_id")
                .and_then(serde_json::Value::as_str)
                .map(|id| format!("Thread {id}"));
            out.push(AgentEvent::Claude(ClaudeEvent::System {
                subtype: "thread_started".to_string(),
                model: Some("codex".to_string()),
                description,
                session_id: None,
                claude_code_version: None,
                tools: None,
            }));
        }
        "turn.started" => {
            out.push(AgentEvent::Claude(ClaudeEvent::System {
                subtype: "turn_started".to_string(),
                model: Some("codex".to_string()),
                description: None,
                session_id: None,
                claude_code_version: None,
                tools: None,
            }));
        }
        "item.started" | "item.completed" => {
            let is_completed = event_type == "item.completed";
            let Some(item) = v.get("item").and_then(serde_json::Value::as_object) else {
                return Some(out);
            };
            let item_id = item
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("codex_item")
                .to_string();
            let item_type = item
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            match item_type {
                "agent_message" => {
                    if is_completed
                        && let Some(text) = item.get("text").and_then(serde_json::Value::as_str)
                    {
                        let text = text.trim();
                        if !text.is_empty() {
                            out.push(assistant_text_event(text.to_string()));
                        }
                    }
                }
                "command_execution" => {
                    let command = item
                        .get("command")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let status = item
                        .get("status")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown");

                    if !is_completed {
                        out.push(assistant_block_event(ContentBlock::ToolUse {
                            id: item_id,
                            name: "command_execution".to_string(),
                            input: serde_json::json!({
                                "command": command,
                                "status": status
                            }),
                        }));
                    } else {
                        let exit_code = item
                            .get("exit_code")
                            .and_then(serde_json::Value::as_i64)
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "?".to_string());
                        let aggregated_output = item
                            .get("aggregated_output")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("")
                            .trim_end();
                        let mut content =
                            format!("$ {command}\nstatus: {status} (exit {exit_code})");
                        if !aggregated_output.is_empty() {
                            content.push_str("\n\n");
                            content.push_str(aggregated_output);
                        }
                        out.push(assistant_block_event(ContentBlock::ToolResult {
                            id: item_id,
                            content,
                        }));
                    }
                }
                "reasoning" => {
                    if is_completed
                        && let Some(text) = item.get("text").and_then(serde_json::Value::as_str)
                    {
                        let text = text.trim();
                        if !text.is_empty() {
                            out.push(assistant_block_event(ContentBlock::Thinking {
                                thinking: text.to_string(),
                            }));
                        }
                    }
                }
                _ => {
                    if is_completed
                        && let Some(text) = item.get("text").and_then(serde_json::Value::as_str)
                    {
                        let text = text.trim();
                        if !text.is_empty() {
                            out.push(assistant_text_event(text.to_string()));
                        }
                    }
                }
            }
        }
        "turn.completed" => {
            let usage = v.get("usage").unwrap_or(&serde_json::Value::Null);
            let input_tokens_u64 = usage
                .get("input_tokens")
                .and_then(serde_json::Value::as_u64);
            let output_tokens_u64 = usage
                .get("output_tokens")
                .and_then(serde_json::Value::as_u64);
            let mut summary_parts = Vec::new();
            if let Some(input) = input_tokens_u64 {
                summary_parts.push(format!("input_tokens={input}"));
            }
            if let Some(output) = output_tokens_u64 {
                summary_parts.push(format!("output_tokens={output}"));
            }
            let summary = if summary_parts.is_empty() {
                None
            } else {
                Some(summary_parts.join(", "))
            };
            out.push(AgentEvent::Claude(ClaudeEvent::Result {
                status: "completed".to_string(),
                summary,
                duration_ms: None,
                input_tokens: u64_to_u32(input_tokens_u64),
                output_tokens: u64_to_u32(output_tokens_u64),
            }));
        }
        _ => {}
    }

    Some(out)
}

fn run_codex_native_with_env(
    args: &[String],
    extra_env: &[(String, String)],
    cwd: Option<&Path>,
) -> bool {
    let mut cmd = Command::new("codex");
    cmd.args(args)
        // Clear API keys so agents use the user's subscription instead of API credits.
        .env("ANTHROPIC_API_KEY", "")
        .env("OPENAI_API_KEY", "")
        .env("GEMINI_API_KEY", "");
    if let Some(p) = cwd {
        cmd.current_dir(p);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|_| panic!("failed to spawn codex"));
    set_active_child_pid(Some(child.id()));

    let stdout = child.stdout.take().expect("piped stdout");
    let reader = BufReader::new(stdout);

    for line in reader.lines().map_while(Result::ok) {
        if stop_requested() {
            let _ = child.kill();
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(events) = codex_events_from_json_line(trimmed) {
            for ev in events {
                emit_event(ev);
            }
        } else {
            log(&format!("codex: {trimmed}"));
        }
    }
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    set_active_child_pid(None);
    ok
}

pub fn run_agent(cfg: &Config, prompt: &str) {
    run_agent_inner(cfg, prompt, &[], None);
}

/// Like [`run_agent`] but injects extra environment variables into the subprocess.
pub fn run_agent_with_env(cfg: &Config, prompt: &str, extra_env: &[(String, String)]) {
    run_agent_inner(cfg, prompt, extra_env, None);
}

/// Like [`run_agent_with_env`] but also pins the agent subprocess's working
/// directory to `cwd`. Used by [`run_pr_review_fix`] (#144) so the agent
/// edits files inside an isolated git worktree without disturbing the dev
/// process's own cwd or the user's main checkout.
pub fn run_agent_with_env_in(
    cfg: &Config,
    prompt: &str,
    extra_env: &[(String, String)],
    cwd: &Path,
) {
    run_agent_inner(cfg, prompt, extra_env, Some(cwd));
}

/// Shared body for [`run_agent`], [`run_agent_with_env`], and
/// [`run_agent_with_env_in`]. The four `cfg.agent` arms differ only in how
/// they assemble argv; the `cwd` parameter is plumbed through to every
/// subprocess so worktree-isolated runs and ordinary runs share one code path.
fn run_agent_inner(cfg: &Config, prompt: &str, extra_env: &[(String, String)], cwd: Option<&Path>) {
    if stop_requested() {
        log("Stop requested; skipping agent run.");
        return;
    }
    if let Some(p) = cwd {
        log(&format!("Running {} in {}...", cfg.agent, p.display()));
    } else {
        log(&format!("Running {}...", cfg.agent));
    }
    let launch_overrides = local_inference_overrides(cfg);
    let merged_env = merged_agent_env(cfg, extra_env);
    let t0 = Instant::now();

    let ok = match cfg.agent {
        Agent::Claude => {
            let mut args = vec![
                "-p".to_string(),
                "--verbose".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
            ];
            args.extend(launch_overrides.args.iter().cloned());
            if cfg.auto_mode {
                args.push("--dangerously-skip-permissions".to_string());
            }
            args.push(prompt.to_string());
            run_claude_native_with_env("claude", &args, &merged_env, cwd)
        }

        Agent::Codex => {
            let mut args = vec!["exec".to_string(), "--json".to_string()];
            args.extend(launch_overrides.args.iter().cloned());
            if cfg.auto_mode {
                args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
            }
            args.push("--".to_string());
            args.push(prompt.to_string());
            run_codex_native_with_env(&args, &merged_env, cwd)
        }

        Agent::Copilot => {
            let mut cmd = Command::new("copilot");
            let mut args = vec!["-p".to_string(), prompt.to_string()];
            if cfg.auto_mode {
                args.push("--yolo".to_string());
            }
            cmd.args(&args);
            if let Some(p) = cwd {
                cmd.current_dir(p);
            }
            for (k, v) in &merged_env {
                cmd.env(k, v);
            }
            cmd.status().map(|s| s.success()).unwrap_or(false)
        }

        Agent::Gemini => {
            let mut args = vec![
                "-p".to_string(),
                prompt.to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
            ];
            if cfg.auto_mode {
                args.push("--yolo".to_string());
            }
            run_claude_native_with_env("gemini", &args, &merged_env, cwd)
        }
    };

    log(&format!("[timing] agent run completed in {:.2?}", t0.elapsed()));

    if !ok {
        if stop_requested() {
            log("Agent run stopped by user request.");
            return;
        }
        die(&format!("{} exited with an error", cfg.agent));
    }
}

/// Generate a cleaned markdown snapshot of the entire codebase using toak-rs.
///
/// Returns the snapshot string, truncated to [`MAX_SNAPSHOT_TOKENS`] if necessary.
pub fn generate_codebase_snapshot(root: &str) -> String {
    log("Generating codebase snapshot with toak-rs...");

    let snapshot_path = PathBuf::from(root).join("prompt.md");
    let opts = MarkdownGeneratorOptions {
        dir: PathBuf::from(root),
        output_file_path: snapshot_path.clone(),
        verbose: false,
        ..Default::default()
    };

    let mut generator = MarkdownGenerator::new(opts);

    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(generator.create_markdown_document())
    });

    // Revert toak side-effects on .gitignore if it was modified.
    let _ = cmd_stdout("git", &["checkout", "--", ".gitignore"]);

    let snapshot = match result {
        Ok(res) if res.success => std::fs::read_to_string(&snapshot_path).unwrap_or_default(),
        Ok(_) => {
            log(
                "WARNING: toak-rs markdown generation reported failure, continuing without snapshot",
            );
            String::new()
        }
        Err(e) => {
            log(&format!(
                "WARNING: toak-rs snapshot failed: {e}, continuing without snapshot"
            ));
            String::new()
        }
    };

    // Clean up the temp file.
    let _ = std::fs::remove_file(&snapshot_path);

    // Truncate if over budget.
    let tokens = count_tokens(&snapshot);
    if tokens > MAX_SNAPSHOT_TOKENS {
        log(&format!(
            "Snapshot is {tokens} tokens, truncating to {MAX_SNAPSHOT_TOKENS}"
        ));
        truncate_snapshot(snapshot, MAX_SNAPSHOT_TOKENS)
    } else {
        log(&format!("Snapshot ready ({tokens} tokens)"));
        snapshot
    }
}

/// Truncate a snapshot string to fit within a token budget.
///
/// Uses a conservative 3-bytes-per-token estimate so the result never exceeds
/// `max_tokens` when re-tokenized.
fn truncate_snapshot(snapshot: String, max_tokens: usize) -> String {
    let max_bytes = max_tokens * 3;
    let truncated = if snapshot.len() > max_bytes {
        let mut end = max_bytes;
        while !snapshot.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &snapshot[..end]
    } else {
        &snapshot
    };
    format!(
        "{truncated}\n\n[... snapshot truncated at {max_tokens} tokens — use `toak` CLI for full exploration ...]"
    )
}

pub fn work_on_issue(cfg: &Config, tracker_num: u32, issue_num: u32, blockers: &[u32]) {
    if stop_requested() {
        return;
    }
    let issue_t0 = Instant::now();
    let (title, body) = fetch_issue(issue_num);
    log(&format!("Issue #{issue_num}: {title}"));

    let tracker_body = if tracker_num != 0 {
        get_tracker_body(tracker_num)
    } else {
        String::new()
    };

    if cfg.dry_run {
        let codebase = if env::var("DISABLE_TOAK").is_ok_and(|v| v == "1") {
            log("Skipping toak snapshot (DISABLE_TOAK=1)");
            String::new()
        } else {
            generate_codebase_snapshot(&cfg.root)
        };
        let prompt = build_prompt(
            &cfg.project_name,
            issue_num,
            &title,
            &body,
            &codebase,
            tracker_num,
            &tracker_body,
        );
        log_resolved_agent_launch(cfg, &[]);
        log(&format!(
            "[dry-run] Prompt ({} tokens). Would work on #{issue_num}, then open PR.\n\n---\n{}",
            toak_rs::count_tokens(&prompt),
            prompt
        ));
        return;
    }

    let branch = format!("{BRANCH_PREFIX}{issue_num}");
    let base = find_upstream_branch(blockers);

    // Start from the upstream dependency branch (or master if no blockers).
    if base != "master" {
        log(&format!("Chaining off upstream branch '{base}'"));
        cmd_run("git", &["fetch", "origin", &base]);
        cmd_run("git", &["checkout", &base]);
        cmd_run("git", &["pull", "origin", &base]);
    } else {
        cmd_run("git", &["checkout", "master"]);
    }
    cmd_run("git", &["branch", "-D", &branch]); // remove stale branch if any
    cmd_run("git", &["checkout", "-b", &branch]);

    let codebase = timed!("snapshot", {
        if env::var("DISABLE_TOAK").is_ok_and(|v| v == "1") {
            log("Skipping toak snapshot (DISABLE_TOAK=1)");
            String::new()
        } else {
            generate_codebase_snapshot(&cfg.root)
        }
    });
    run_agent(
        cfg,
        &build_prompt(
            &cfg.project_name,
            issue_num,
            &title,
            &body,
            &codebase,
            tracker_num,
            &tracker_body,
        ),
    );
    if stop_requested() {
        log("Stop requested; halting issue workflow before tests/commit.");
        return;
    }

    timed!("tests", {
        log("Running tests...");
        if !cmd_run("./scripts/test-examples.sh", &[]) {
            log(&format!(
                "Tests failed for #{issue_num} — invoking agent to fix..."
            ));
            let (_, test_out) =
                cmd_capture("cargo", &["test", "--workspace", "--exclude", "hello-wasm"]);
            let fix_prompt = build_test_fix_prompt(issue_num, &test_out);
            run_agent(cfg, &fix_prompt);
            cmd_run("cargo", &["fmt", "--all"]);
        }
    });

    let commit_msg = format!(
        "implement #{issue_num}: {title}\n\nCloses #{issue_num}\n\n{}",
        cfg.agent.co_author()
    );
    timed!("commit", commit_with_retries(cfg, issue_num, &branch, &commit_msg));

    // Push the branch and open a pull request targeting the upstream branch.
    // The pre-push hook runs `cargo test`, so failures here mean test failures.
    // Retry by invoking the agent to fix the failing tests, then recommit and push.
    timed!("push", push_with_retries(cfg, issue_num, &branch, &commit_msg));

    timed!("pr-create", {
        let pr_body_text = format!("Closes #{issue_num}\n\n{}", cfg.agent.co_author());
        let (pr_ok, pr_out) = cmd_capture(
            "gh",
            &[
                "pr",
                "create",
                "--title",
                &format!("#{issue_num}: {title}"),
                "--body",
                &pr_body_text,
                "--base",
                &base,
                "--head",
                &branch,
            ],
        );
        if pr_ok {
            log(&format!("Opened PR for #{issue_num}: {}", pr_out.trim()));
        } else {
            log(&format!(
                "WARNING: failed to create PR for #{issue_num}: {pr_out}"
            ));
        }
    });

    // Return to master for the next iteration.
    cmd_run("git", &["checkout", "master"]);
    log(&format!(
        "[timing] issue #{issue_num} total: {:.2?}",
        issue_t0.elapsed()
    ));
}

pub fn commit_with_retries(cfg: &Config, issue_num: u32, branch: &str, msg: &str) {
    for i in 1..=MAX_COMMIT_ATTEMPTS {
        if stop_requested() {
            log("Stop requested; skipping commit retries.");
            return;
        }
        // Check if there is anything to commit before attempting.
        let (_, status_out) = cmd_capture("git", &["status", "--porcelain"]);
        if status_out.trim().is_empty() {
            // Working tree clean — the AI agent likely committed already.
            log(&format!(
                "Nothing to commit for #{issue_num} — changes already committed by agent."
            ));
            return;
        }

        if cmd_run("git", &["commit", "-am", msg]) {
            log(&format!("Committed changes for #{issue_num} (attempt {i})"));
            return;
        }
        log(&format!(
            "Commit attempt {i} failed for #{issue_num}, auto-fixing and retrying..."
        ));

        // Step 1: auto-fix what tooling can handle.
        cmd_run("cargo", &["fmt", "--all"]);
        cmd_run(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--fix",
                "--allow-dirty",
                "--allow-staged",
            ],
        );
        cmd_run("git", &["add", "-A"]);

        // Step 2: if clippy still has warnings, invoke the AI agent to fix them.
        let (clippy_ok, clippy_out) = cmd_capture(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        );
        if !clippy_ok {
            log("Clippy still has warnings after auto-fix, invoking agent to resolve...");
            let fix_prompt = build_lint_fix_prompt(issue_num, &clippy_out);
            run_agent(cfg, &fix_prompt);
            cmd_run("cargo", &["fmt", "--all"]);
            cmd_run("git", &["add", "-A"]);
        }
    }
    die(&format!(
        "failed to commit changes for #{issue_num} after {MAX_COMMIT_ATTEMPTS} attempts. Branch '{branch}' left for inspection."
    ));
}

pub fn push_with_retries(cfg: &Config, issue_num: u32, branch: &str, commit_msg: &str) {
    for i in 1..=MAX_PUSH_ATTEMPTS {
        if stop_requested() {
            log("Stop requested; skipping push retries.");
            return;
        }

        let (ok, _push_out) = cmd_capture("git", &["push", "-u", "origin", branch]);
        if ok {
            log(&format!("Pushed branch '{branch}' (attempt {i})"));
            return;
        }

        log(&format!(
            "Push attempt {i}/{MAX_PUSH_ATTEMPTS} failed for #{issue_num} — invoking agent to fix test failures..."
        ));

        // Run `cargo test` separately to get clean test output for the agent.
        let (_, test_out) =
            cmd_capture("cargo", &["test", "--workspace", "--exclude", "hello-wasm"]);

        let fix_prompt = build_test_fix_prompt(issue_num, &test_out);
        run_agent(cfg, &fix_prompt);

        // Re-format, stage, and commit the fixes.
        cmd_run("cargo", &["fmt", "--all"]);
        cmd_run("git", &["add", "-A"]);

        let (_, status_out) = cmd_capture("git", &["status", "--porcelain"]);
        if !status_out.trim().is_empty() {
            let fix_msg = format!("{commit_msg}\n\n[auto-fix: test failures, attempt {i}]");
            if !cmd_run("git", &["commit", "-am", &fix_msg]) {
                log(&format!(
                    "Commit after test fix attempt {i} failed, running commit_with_retries..."
                ));
                commit_with_retries(cfg, issue_num, branch, commit_msg);
            }
        }
    }
    die(&format!(
        "failed to push branch '{branch}' after {MAX_PUSH_ATTEMPTS} attempts. Branch left for inspection."
    ));
}

pub fn preflight(cfg: &Config) {
    if !has_command("gh") {
        die("`gh` CLI not found. Please install GitHub CLI.");
    }
    if !has_command(cfg.agent.binary()) {
        die(&format!(
            "`{}` binary not found. Please ensure it is in your PATH.",
            cfg.agent.binary()
        ));
    }
    if cfg.bootstrap_agent_files {
        ensure_agent_files(cfg);
    }
}

/// Ensure `AGENTS.md` and standard skills exist on disk, bootstrapping missing
/// files from compile-time defaults. Existing repo files remain authoritative.
fn ensure_agent_files(cfg: &Config) {
    let root = Path::new(&cfg.root);

    // 1. Ensure AGENTS.md exists.
    let agents_md_path = root.join("AGENTS.md");
    if !agents_md_path.exists() {
        log("Initializing missing AGENTS.md from embedded defaults...");
        let _ = std::fs::write(&agents_md_path, AGENTS_MD.as_bytes());
    }

    // 2. Ensure standard skills and their bundled support files exist.
    for file in SkillAssets::iter() {
        let path = root.join(".agents/skills").join(file.as_ref());
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Some(embedded) = SkillAssets::get(file.as_ref()) {
                log(&format!(
                    "Initializing missing skill asset `{}` from embedded defaults...",
                    file
                ));
                let _ = std::fs::write(&path, embedded.data);
            }
        }
    }
}

pub fn run_loop(cfg: &Config, tracker_num: u32) {
    preflight(cfg);

    // Phase 1: drain any open "retro:" action items first.
    loop {
        if stop_requested() {
            log("Stop requested. Exiting run loop.");
            if let Some(tx) = EVENT_SENDER.get() {
                let _ = tx.send(AgentEvent::Done);
            }
            return;
        }
        let retro = find_retro_issues();
        if retro.is_empty() {
            break;
        }
        let issue_num = retro[0];
        log(&format!("Working on retro item #{issue_num}..."));
        work_on_issue(cfg, tracker_num, issue_num, &[]);
        close_issue(issue_num);
    }

    // Phase 2: work through the sprint tracker checklist.
    loop {
        if stop_requested() {
            log("Stop requested. Exiting run loop.");
            if let Some(tx) = EVENT_SENDER.get() {
                let _ = tx.send(AgentEvent::Done);
            }
            return;
        }
        let body = get_tracker_body(tracker_num);
        let completed = parse_completed(&body);
        let pending = parse_pending(&body);

        // Push the current issue list to the UI so it stays in sync.
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::TrackerUpdate(pending.clone()));
        }

        let Some(next) = pending.into_iter().find(|i| is_ready(i, &completed)) else {
            log("No more ready issues. Done!");
            if let Some(tx) = EVENT_SENDER.get() {
                let _ = tx.send(AgentEvent::Done);
            }
            break;
        };

        work_on_issue(cfg, tracker_num, next.number, &next.blockers);
        if stop_requested() {
            log("Stop requested. Leaving tracker untouched for current issue.");
            if let Some(tx) = EVENT_SENDER.get() {
                let _ = tx.send(AgentEvent::Done);
            }
            return;
        }
        check_off_issue(tracker_num, next.number);
    }
}

/// Run the implementation workflow for a single issue, dispatched directly
/// from the ISSUES sidebar list rather than from the tracker walk loop.
///
/// Unlike `run_loop`, this does not consult or check off any tracker — the
/// caller has bypassed the dependency-ordered queue and asked for one
/// specific issue. The PR opened by `work_on_issue` still includes
/// `Closes #N`, so the issue auto-closes on merge; any tracker checkbox
/// will catch up on the next tracker walk.
pub fn run_single_issue(cfg: &Config, issue_num: u32) {
    preflight(cfg);
    log(&format!("Starting work on issue #{issue_num}..."));
    work_on_issue(cfg, 0, issue_num, &[]);
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.send(AgentEvent::Done);
    }
}

// ── Generic YAML-driven workflow runners ──

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

// ── Sprint planning (two-phase) ──

pub fn run_sprint_planning_draft(cfg: &Config) {
    run_workflow_draft(cfg, "sprint_planning");
}

pub fn run_sprint_planning_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "sprint_planning", feedback);
}

/// Gather strategic context as a tuple (used by interview workflow which
/// hasn't been migrated to YAML templates yet).
fn gather_strategic_context_base(cfg: &Config) -> (String, String, String, String, String, String) {
    let ctx = crate::agent::workflow::gather_context_as_json(cfg, "strategic");
    (
        ctx["open_issues"].as_str().unwrap_or("[]").to_string(),
        ctx["open_prs"].as_str().unwrap_or("[]").to_string(),
        ctx["recent_commits"].as_str().unwrap_or("").to_string(),
        ctx["crate_tree"].as_str().unwrap_or("").to_string(),
        ctx["status"].as_str().unwrap_or("").to_string(),
        ctx["issues_md"].as_str().unwrap_or("").to_string(),
    )
}

// ── Strategic review (two-phase) ──

pub fn run_strategic_review_draft(cfg: &Config) {
    run_workflow_draft(cfg, "strategic_review");
}

pub fn run_strategic_review_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "strategic_review", feedback);
}

// ── Roadmapper (two-phase) ──

pub fn run_roadmapper_draft(cfg: &Config) {
    run_workflow_draft(cfg, "roadmapper");
}

pub fn run_roadmapper_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "roadmapper", feedback);
}

// ── Ideation (two-phase) ──

pub fn run_ideation_draft(cfg: &Config) {
    run_workflow_draft(cfg, "ideation");
}

pub fn run_ideation_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "ideation", feedback);
}

// ── UXR Synth (two-phase) ──

pub fn run_report_draft(cfg: &Config) {
    run_workflow_draft(cfg, "report_research");
}

pub fn run_report_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "report_research", feedback);
}

// ── Retrospective (two-phase) ──

pub fn run_retrospective_draft(cfg: &Config) {
    run_workflow_draft(cfg, "retrospective");
}

pub fn run_retrospective_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "retrospective", feedback);
}

// ── Housekeeping (two-phase) ──

pub fn run_housekeeping_draft(cfg: &Config) {
    run_workflow_draft(cfg, "housekeeping");
}

pub fn run_housekeeping_finalize(cfg: &Config, feedback: &str) {
    run_workflow_finalize(cfg, "housekeeping", feedback);
}

// ── Security Review (one-shot) ──

pub fn run_security_code_review(cfg: &Config) {
    preflight(cfg);
    log("Starting security-focused code review...");

    let crate_tree = cmd_stdout("ls", &["-1", &format!("{}/crates", cfg.root)]).unwrap_or_default();

    let codebase = if env::var("DISABLE_TOAK").is_ok_and(|v| v == "1") {
        log("Skipping toak snapshot (DISABLE_TOAK=1)");
        String::new()
    } else {
        generate_codebase_snapshot(&cfg.root)
    };

    let prompt =
        build_security_review_prompt(&cfg.project_name, &crate_tree, &codebase, cfg.dry_run);

    if cfg.dry_run {
        log_resolved_agent_launch(cfg, &[]);
        log("[dry-run] Running security review analysis (no issues will be filed)...");
    }

    run_agent(cfg, &prompt);
    if stop_requested() {
        log("Stop requested. Security review cancelled.");
        emit_event(AgentEvent::Done);
        return;
    }

    log("Security review complete.");
    emit_event(AgentEvent::Done);
}

// ── Refresh Agents (one-shot) ──

/// Enumerate agent-facing files: AGENTS.md, .agents/skills/*/SKILL.md,
/// and optional vendor files (CLAUDE.md, GEMINI.md, COPILOT.md).
fn enumerate_agent_files(root: &str) -> Vec<String> {
    let root_path = Path::new(root);
    let mut files = BTreeSet::new();

    let agents_md = root_path.join("AGENTS.md");
    if agents_md.exists() {
        files.insert("AGENTS.md".to_string());
    }

    let skills_dir = root_path.join(".agents/skills");
    if skills_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&skills_dir)
    {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let skill_md = entry.path().join("SKILL.md");
                if skill_md.exists()
                    && let Ok(rel) = skill_md.strip_prefix(root_path)
                {
                    files.insert(rel.to_string_lossy().to_string());
                }
            }
        }
    }

    for name in &["CLAUDE.md", "GEMINI.md", "COPILOT.md"] {
        let p = root_path.join(name);
        if p.exists() {
            files.insert(name.to_string());
        }
    }

    files.into_iter().collect()
}

pub fn run_refresh_agents(cfg: &Config) {
    preflight(cfg);
    log("Starting Refresh Agents...");

    let agent_files = enumerate_agent_files(&cfg.root);
    if agent_files.is_empty() {
        log("No agent-facing files found — nothing to refresh.");
        emit_event(AgentEvent::Done);
        return;
    }

    log(&format!("Found {} agent-facing file(s)", agent_files.len()));

    let prompt = build_refresh_agents_prompt(&cfg.project_name, &agent_files);

    if cfg.dry_run {
        log("[dry-run] Would run Refresh Agents to review agent-facing docs");
        log(&format!(
            "[dry-run] Files in scope: {}",
            agent_files.join(", ")
        ));
        log(&format!("[dry-run] Prompt length: {} chars", prompt.len()));
        emit_event(AgentEvent::Done);
        return;
    }

    // Create a working branch.
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let branch = format!("{BRANCH_PREFIX}refresh-agents-{ts}");
    cmd_run("git", &["checkout", "master"]);
    cmd_run("git", &["branch", "-D", &branch]);
    cmd_run("git", &["checkout", "-b", &branch]);

    run_agent(cfg, &prompt);
    if stop_requested() {
        log("Stop requested. Refresh Agents cancelled.");
        cmd_run("git", &["checkout", "master"]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Check if the agent made any changes.
    let (_, status_out) = cmd_capture("git", &["status", "--porcelain"]);
    if status_out.trim().is_empty() {
        log("No drift detected — agent-facing docs are up to date.");
        cmd_run("git", &["checkout", "master"]);
        cmd_run("git", &["branch", "-D", &branch]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Build a diff summary for the PR description.
    // Use `git status --porcelain` instead of `git diff --name-only` so that
    // untracked files (e.g. a newly created SKILL.md) also appear in the list.
    let changed: Vec<&str> = status_out
        .lines()
        .filter_map(|l| l.get(3..))
        .filter(|l| !l.trim().is_empty())
        .collect();
    let mut pr_body = String::from("## Refresh Agents — documentation drift fixes\n\n");
    pr_body.push_str(
        "This PR updates agent-facing documentation to match the current repo state.\n\n",
    );
    pr_body.push_str("### Files changed\n\n");
    for f in &changed {
        pr_body.push_str(&format!("- `{f}`\n"));
    }
    pr_body.push_str(&format!("\n{}", cfg.agent.co_author()));

    // Stage only the enumerated agent files (not the entire working tree).
    for f in &agent_files {
        cmd_run("git", &["add", f]);
    }

    // Warn about out-of-scope modifications the agent may have introduced.
    let (_, all_status) = cmd_capture("git", &["status", "--porcelain"]);
    let out_of_scope: Vec<&str> = all_status
        .lines()
        .filter_map(|line| {
            // Unstaged/untracked entries start with ' ' or '?' in the first column.
            // We care about modified-but-not-staged files (agent touched something it shouldn't).
            let file = line.get(3..)?.trim();
            if file.is_empty() {
                return None;
            }
            if agent_files.iter().any(|af| af.as_str() == file) {
                return None;
            }
            // Only flag files with working-tree modifications (not already staged).
            let index_status = line.as_bytes().first().copied().unwrap_or(b' ');
            let wt_status = line.as_bytes().get(1).copied().unwrap_or(b' ');
            if wt_status != b' ' && wt_status != b'?' && index_status == b' ' {
                Some(file)
            } else if wt_status == b'?' {
                // Untracked file the agent may have created.
                Some(file)
            } else {
                None
            }
        })
        .collect();
    if !out_of_scope.is_empty() {
        log(&format!(
            "WARNING: out-of-scope files modified (not staged): {}",
            out_of_scope.join(", ")
        ));
    }

    // Commit.
    let commit_msg = format!("refresh agent-facing docs\n\n{}", cfg.agent.co_author());
    if !cmd_run("git", &["commit", "-m", &commit_msg]) {
        log("WARNING: commit failed — leaving branch for inspection.");
        cmd_run("git", &["checkout", "master"]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Push.
    let (push_ok, push_out) = cmd_capture("git", &["push", "-u", "origin", &branch]);
    if !push_ok {
        log(&format!("WARNING: push failed: {push_out}"));
        cmd_run("git", &["checkout", "master"]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Create PR.
    let (pr_ok, pr_out) = cmd_capture(
        "gh",
        &[
            "pr",
            "create",
            "--title",
            "Refresh Agents: update agent-facing docs",
            "--body",
            &pr_body,
            "--base",
            "master",
            "--head",
            &branch,
        ],
    );
    if pr_ok {
        log(&format!("Opened PR: {}", pr_out.trim()));
    } else {
        log(&format!("WARNING: failed to create PR: {pr_out}"));
    }

    cmd_run("git", &["checkout", "master"]);
    log("Refresh Agents complete.");
    emit_event(AgentEvent::Done);
}

// ── Refresh Docs (one-shot) ──

/// Run `git status --porcelain` scoped to the given paths. When `cwd` is
/// `Some`, the command runs in that directory; otherwise it uses the inherited
/// working directory. Returns raw porcelain output (empty on failure).
///
/// Path scoping (`-- path1 path2 ...`) is critical for the Refresh Docs
/// no-op detection: a whole-repo `git status` would mistake unrelated dirty
/// worktree state for documentation drift.
fn git_status_porcelain_scoped(cwd: Option<&Path>, paths: &[String]) -> String {
    let mut cmd = Command::new("git");
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    cmd.args(["status", "--porcelain", "--"]);
    for p in paths {
        cmd.arg(p);
    }
    cmd.output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// Return the list of paths currently staged in the git index
/// (`git diff --cached --name-only`).
fn git_staged_files(cwd: Option<&Path>) -> Vec<String> {
    let mut cmd = Command::new("git");
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    cmd.args(["diff", "--cached", "--name-only"]);
    let out = cmd
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    out.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Commit only the given paths, even if other files are already staged.
fn git_commit_paths(cwd: Option<&Path>, commit_msg: &str, paths: &[String]) -> bool {
    let mut cmd = Command::new("git");
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    cmd.args(["commit", "-m", commit_msg, "--"]);
    for path in paths {
        cmd.arg(path);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// Enumerate project documentation files: README.md, STATUS.md, ISSUES.md,
/// and all files under docs/.
fn enumerate_project_doc_files(root: &str) -> Vec<String> {
    let root_path = Path::new(root);
    let mut files = BTreeSet::new();

    for name in &["README.md", "STATUS.md", "ISSUES.md"] {
        let p = root_path.join(name);
        if p.exists() {
            files.insert(name.to_string());
        }
    }

    let docs_dir = root_path.join("docs");
    if docs_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&docs_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                && let Ok(rel) = path.strip_prefix(root_path)
            {
                files.insert(rel.to_string_lossy().to_string());
            }
        }
    }

    files.into_iter().collect()
}

pub fn run_refresh_docs(cfg: &Config) {
    preflight(cfg);
    log("Starting Refresh Docs...");

    let doc_files = enumerate_project_doc_files(&cfg.root);
    if doc_files.is_empty() {
        log("No project documentation files found — nothing to refresh.");
        emit_event(AgentEvent::Done);
        return;
    }

    log(&format!("Found {} project doc file(s)", doc_files.len()));

    let prompt = build_refresh_docs_prompt(&cfg.project_name, &doc_files);

    if cfg.dry_run {
        log("[dry-run] Would run Refresh Docs to review project documentation");
        log(&format!(
            "[dry-run] Files in scope: {}",
            doc_files.join(", ")
        ));
        log(&format!("[dry-run] Prompt length: {} chars", prompt.len()));
        emit_event(AgentEvent::Done);
        return;
    }

    // Create a working branch.
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let branch = format!("{BRANCH_PREFIX}refresh-docs-{ts}");
    cmd_run("git", &["checkout", "master"]);
    cmd_run("git", &["branch", "-D", &branch]);
    cmd_run("git", &["checkout", "-b", &branch]);

    run_agent(cfg, &prompt);
    if stop_requested() {
        log("Stop requested. Refresh Docs cancelled.");
        cmd_run("git", &["checkout", "master"]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Detect drift scoped to the enumerated doc files only. A whole-repo
    // `git status` would treat unrelated dirty worktree state as drift and
    // fall through into a failing commit (#140).
    let scoped_status_out = git_status_porcelain_scoped(None, &doc_files);
    if scoped_status_out.trim().is_empty() {
        log("No drift detected — project docs are up to date.");
        cmd_run("git", &["checkout", "master"]);
        cmd_run("git", &["branch", "-D", &branch]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Build a diff summary for the PR description from the scoped output.
    let changed: Vec<&str> = scoped_status_out
        .lines()
        .filter_map(|l| l.get(3..))
        .filter(|l| !l.trim().is_empty())
        .collect();
    let mut pr_body = String::from("## Refresh Docs — project documentation drift fixes\n\n");
    pr_body.push_str("This PR updates project documentation to match the current repo state.\n\n");
    pr_body.push_str("### Files changed\n\n");
    for f in &changed {
        pr_body.push_str(&format!("- `{f}`\n"));
    }
    pr_body.push_str(&format!("\n{}", cfg.agent.co_author()));

    // Stage only the enumerated doc files (not the entire working tree).
    for f in &doc_files {
        cmd_run("git", &["add", "--", f]);
    }

    // Surface already-staged local changes that will remain outside the
    // docs-only commit. The commit below is pathspec-scoped, so these paths
    // stay staged locally instead of being pulled into the Refresh Docs PR.
    let staged_now = git_staged_files(None);
    let unexpected: Vec<String> = staged_now
        .iter()
        .filter(|f| !doc_files.iter().any(|d| d == *f))
        .cloned()
        .collect();
    if !unexpected.is_empty() {
        log(&format!(
            "WARNING: out-of-scope files already staged locally and excluded from the Refresh Docs commit: {}",
            unexpected.join(", ")
        ));
    }

    // Warn about out-of-scope modifications the agent may have introduced
    // in the working tree (these are not staged and won't be committed,
    // but the user should know about them).
    let (_, all_status) = cmd_capture("git", &["status", "--porcelain"]);
    let out_of_scope: Vec<&str> = all_status
        .lines()
        .filter_map(|line| {
            let file = line.get(3..)?.trim();
            if file.is_empty() {
                return None;
            }
            if doc_files.iter().any(|df| df.as_str() == file) {
                return None;
            }
            let index_status = line.as_bytes().first().copied().unwrap_or(b' ');
            let wt_status = line.as_bytes().get(1).copied().unwrap_or(b' ');
            if wt_status == b'?' || wt_status != b' ' && index_status == b' ' {
                Some(file)
            } else {
                None
            }
        })
        .collect();
    if !out_of_scope.is_empty() {
        log(&format!(
            "WARNING: out-of-scope files modified (not staged): {}",
            out_of_scope.join(", ")
        ));
    }

    // Commit.
    let commit_msg = format!("refresh project docs\n\n{}", cfg.agent.co_author());
    if !git_commit_paths(None, &commit_msg, &doc_files) {
        log("WARNING: commit failed — leaving branch for inspection.");
        cmd_run("git", &["checkout", "master"]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Push.
    let (push_ok, push_out) = cmd_capture("git", &["push", "-u", "origin", &branch]);
    if !push_ok {
        log(&format!("WARNING: push failed: {push_out}"));
        cmd_run("git", &["checkout", "master"]);
        emit_event(AgentEvent::Done);
        return;
    }

    // Create PR.
    let (pr_ok, pr_out) = cmd_capture(
        "gh",
        &[
            "pr",
            "create",
            "--title",
            "Refresh Docs: update project documentation",
            "--body",
            &pr_body,
            "--base",
            "master",
            "--head",
            &branch,
        ],
    );
    if pr_ok {
        log(&format!("Opened PR: {}", pr_out.trim()));
    } else {
        log(&format!("WARNING: failed to create PR: {pr_out}"));
    }

    cmd_run("git", &["checkout", "master"]);
    log("Refresh Docs complete.");
    emit_event(AgentEvent::Done);
}

// ── Interview (multi-round) ──

/// Maximum number of follow-up rounds before the summary is generated.
/// Round 0 = initial questions, rounds 1..MAX = follow-ups, then summary.
const INTERVIEW_MAX_FOLLOWUP_ROUNDS: usize = 1;

pub fn run_interview_draft(cfg: &Config) {
    preflight(cfg);
    log("Starting interview — analyzing project state...");

    // Reset interview state.
    if let Ok(mut answers) = INTERVIEW_ANSWERS.lock() {
        answers.clear();
    }

    let (open_issues, open_prs, recent_commits, crate_tree, status, issues_md) =
        gather_strategic_context_base(cfg);

    let prompt = build_interview_draft_prompt(
        &open_issues,
        &open_prs,
        &recent_commits,
        &status,
        &issues_md,
        &crate_tree,
    );

    if cfg.dry_run {
        log_resolved_agent_launch(cfg, &[]);
        log("[dry-run] Would run interview draft");
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::Done);
        }
        return;
    }

    run_agent(cfg, &prompt);
    if stop_requested() {
        log("Stop requested. Interview cancelled.");
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::Done);
        }
        return;
    }

    log("Review the questions above and provide your answers.");
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.send(AgentEvent::AwaitingFeedback(Workflow::Interview));
    }
}

pub fn run_interview_respond(cfg: &Config, answer: &str) {
    preflight(cfg);

    // Accumulate the answer.
    let answers = {
        let mut guard = INTERVIEW_ANSWERS.lock().unwrap();
        guard.push(answer.to_string());
        guard.clone()
    };

    let round = answers.len(); // 1-indexed (1 = first follow-up, etc.)

    let (open_issues, open_prs, recent_commits, crate_tree, status, issues_md) =
        gather_strategic_context_base(cfg);

    if round <= INTERVIEW_MAX_FOLLOWUP_ROUNDS {
        // Follow-up round.
        log(&format!(
            "Processing answer (round {round}) — generating follow-up questions..."
        ));

        let prompt = build_interview_followup_prompt(
            &open_issues,
            &open_prs,
            &recent_commits,
            &status,
            &issues_md,
            &crate_tree,
            &answers,
        );

        if cfg.dry_run {
            log_resolved_agent_launch(cfg, &[]);
            log(&format!("[dry-run] Would run interview round {round}"));
            if let Some(tx) = EVENT_SENDER.get() {
                let _ = tx.send(AgentEvent::Done);
            }
            return;
        }

        run_agent(cfg, &prompt);
        if stop_requested() {
            log("Stop requested. Interview cancelled.");
            if let Some(tx) = EVENT_SENDER.get() {
                let _ = tx.send(AgentEvent::Done);
            }
            return;
        }

        log("Review the follow-up questions and provide your answers.");
        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::AwaitingFeedback(Workflow::Interview));
        }
    } else {
        // Summary round.
        log("Generating interview summary...");

        let prompt = build_interview_summary_prompt(
            &open_issues,
            &open_prs,
            &recent_commits,
            &status,
            &issues_md,
            &crate_tree,
            &answers,
        );

        if cfg.dry_run {
            log_resolved_agent_launch(cfg, &[]);
            log("[dry-run] Would run interview summary");
            if let Some(tx) = EVENT_SENDER.get() {
                let _ = tx.send(AgentEvent::Done);
            }
            return;
        }

        run_agent(cfg, &prompt);
        if stop_requested() {
            log("Stop requested. Interview summary cancelled.");
        } else {
            log("Interview complete — summary generated above.");
        }

        // Clear state.
        if let Ok(mut guard) = INTERVIEW_ANSWERS.lock() {
            guard.clear();
        }

        if let Some(tx) = EVENT_SENDER.get() {
            let _ = tx.send(AgentEvent::Done);
        }
    }
}

pub fn run_code_review(cfg: &Config) {
    preflight(cfg);
    log("Starting code review...");

    // Resolve bot token so the review subprocess runs under the bot identity.
    let bot_token = cfg.effective_bot_credentials().as_ref().and_then(resolve_bot_token);

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

// ── Phase 2: per-PR Fix Comments dispatch (#144) ──────────────────────────

/// RAII guard that removes a git worktree on drop, including on panic.
///
/// Locked-in #144 design: clicking the Fix Comments button must never leave
/// stale worktrees behind, on any code path. We can't rely on a try/finally
/// idiom in Rust, so a Drop guard is the only way to guarantee cleanup runs
/// even if the surrounding function panics or returns from a deeply-nested
/// match arm.
struct WorktreeGuard {
    path: PathBuf,
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let path_str = self.path.to_string_lossy().to_string();
        // Best-effort cleanup; never panic from Drop. `git worktree remove`
        // is the canonical path and unlinks both the directory and the
        // bookkeeping under .git/worktrees.
        if !cmd_run("git", &["worktree", "remove", "--force", &path_str]) {
            log(&format!(
                "WARNING: `git worktree remove` failed for {path_str}; falling back to fs cleanup"
            ));
            let _ = std::fs::remove_dir_all(&self.path);
            // Prune dangling .git/worktrees entries left over from the
            // failed remove so the next Fix run starts clean.
            let _ = cmd_run("git", &["worktree", "prune"]);
        }
    }
}

/// Phase 2 of the per-row agent dispatch work (#144): dispatch the agent to
/// address unresolved review comments on a single pull request.
///
/// Workflow (mirrors the in-scope list in the issue body):
/// 1. Resolve the PR's title, head branch, and unresolved bot-authored
///    review threads. If there are zero actionable threads, log it and
///    bail out cleanly without touching git state.
/// 2. Create an isolated git worktree on a fresh checkout of the PR's head
///    branch under the system temp dir. A `WorktreeGuard` ensures the
///    worktree is removed on every code path including panic, so the user's
///    main checkout is never disturbed.
/// 3. Run the agent against that worktree (cwd pinned via
///    [`run_agent_with_env_in`]) with a prompt that includes the PR diff
///    and each unresolved thread's `(path, line, body)`.
/// 4. If the agent made changes, commit and push them back to the PR's
///    head branch. If the agent made zero changes, log it and stop.
/// 5. Surface push failures verbatim and stop — no blind retry, per the
///    acceptance criteria.
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

    // Inner function so a single trailing `emit_event(Done)` covers every
    // early return inside `do_pr_review_fix`. The WorktreeGuard's Drop runs
    // when control leaves `do_pr_review_fix`, so cleanup always happens
    // before we tell the UI we're done.
    do_pr_review_fix(cfg, pr_num);
    emit_event(AgentEvent::Done);
}

fn do_pr_review_fix(cfg: &Config, pr_num: u32) {
    let pr_num_s = pr_num.to_string();

    let title = match cmd_stdout(
        "gh",
        &["pr", "view", &pr_num_s, "--json", "title", "--jq", ".title"],
    ) {
        Some(t) if !t.is_empty() => t,
        _ => {
            log(&format!("ERROR: could not fetch PR #{pr_num} title"));
            return;
        }
    };
    let branch = pr_head_branch(pr_num);
    log(&format!("PR #{pr_num} '{title}' — head branch '{branch}'"));

    let threads = fetch_unresolved_review_threads(pr_num, DEFAULT_REVIEW_BOT_LOGIN);
    if threads.is_empty() {
        log(&format!(
            "no unresolved threads on PR #{pr_num} — nothing to fix."
        ));
        return;
    }
    log(&format!(
        "Found {} unresolved thread(s) on PR #{pr_num}",
        threads.len()
    ));

    // Prune any orphaned worktrees from prior crashed runs before claiming a
    // new path. Cheap and idempotent.
    let _ = cmd_run("git", &["worktree", "prune"]);

    // Build a unique worktree path under the system temp dir so concurrent
    // Fix runs on different PRs don't collide. The nonce is wall-clock time
    // in seconds, which is fine because the same PR can only have one Fix
    // run in flight at a time (the sidebar Fix button is disabled while
    // `working` is true).
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let worktree_path = std::env::temp_dir().join(format!("freq-ai-prfix-{pr_num}-{nonce}"));
    let worktree_str = worktree_path.to_string_lossy().to_string();

    // Fetch the branch tip so we check out the latest origin state, not a
    // stale local ref. If origin can't be reached we bail before creating
    // the worktree so there's nothing to clean up.
    if !cmd_run("git", &["fetch", "origin", &branch]) {
        log(&format!("ERROR: failed to fetch origin/{branch}"));
        return;
    }

    // Detached worktree on origin/<branch>. We then `switch -C <branch>` to
    // pin a local branch name inside the worktree so the eventual push
    // target is unambiguous regardless of any local branch ref the user
    // might already have for this PR. `--force` lets us reuse a stale
    // worktree path or check out a branch that's already in another
    // worktree (uncommon but valid).
    let origin_ref = format!("origin/{branch}");
    if !cmd_run(
        "git",
        &[
            "worktree",
            "add",
            "--force",
            "--detach",
            &worktree_str,
            &origin_ref,
        ],
    ) {
        log(&format!(
            "ERROR: failed to create worktree at {worktree_str}"
        ));
        return;
    }

    // Guard takes ownership of cleanup from this point on. Anything below
    // that returns or panics will trigger Drop, which removes the worktree.
    let _wt_guard = WorktreeGuard {
        path: worktree_path.clone(),
    };

    log(&format!("Worktree created at {worktree_str}"));

    if !cmd_run("git", &["-C", &worktree_str, "switch", "-C", &branch]) {
        log(&format!(
            "ERROR: failed to switch worktree to branch {branch}"
        ));
        return;
    }

    // Use `gh pr diff` for the prompt diff so it matches the GitHub-side
    // view of the PR (same merge base the reviewer saw).
    let diff = pr_diff(pr_num);

    let prompt =
        build_pr_review_fix_prompt(&cfg.project_name, pr_num, &title, &branch, &diff, &threads);

    // Resolve bot token so any `gh` calls the agent makes (e.g. fetching
    // additional PR context) run as the bot. The push itself uses the
    // user's git credentials, which is fine — clicking the Fix button is
    // an explicit authorization to push.
    let bot_token = cfg.effective_bot_credentials().as_ref().and_then(resolve_bot_token);
    let extra_env: Vec<(String, String)> = bot_token
        .as_deref()
        .map(|t| vec![("GH_TOKEN".to_string(), t.to_string())])
        .unwrap_or_default();

    run_agent_with_env_in(cfg, &prompt, &extra_env, &worktree_path);
    if stop_requested() {
        log("Stop requested. Fix Comments cancelled.");
        return;
    }

    // Did the agent actually edit anything?
    let (_, status_out) = cmd_capture("git", &["-C", &worktree_str, "status", "--porcelain"]);
    if status_out.trim().is_empty() {
        log(&format!(
            "Agent made no changes for PR #{pr_num} — nothing to commit."
        ));
        return;
    }

    if !cmd_run("git", &["-C", &worktree_str, "add", "-A"]) {
        log(&format!(
            "ERROR: failed to stage changes in worktree for PR #{pr_num}"
        ));
        return;
    }
    let commit_msg = format!(
        "fix: address review comments on PR #{pr_num}\n\n{}",
        cfg.agent.co_author()
    );
    if !cmd_run("git", &["-C", &worktree_str, "commit", "-m", &commit_msg]) {
        log(&format!(
            "ERROR: failed to commit Fix Comments changes for PR #{pr_num}"
        ));
        return;
    }
    log(&format!("Committed fix in worktree for PR #{pr_num}"));

    // Push HEAD to origin/<branch>. Using `HEAD:<branch>` so we don't depend
    // on the worktree's local branch having an upstream configured.
    let push_target = format!("HEAD:{branch}");
    let (push_ok, push_out) = cmd_capture(
        "git",
        &["-C", &worktree_str, "push", "origin", &push_target],
    );
    if !push_ok {
        // Acceptance criterion: surface push failures verbatim and stop, no
        // blind retry. The full output goes to the log so the user can see
        // exactly what GitHub rejected.
        log(&format!(
            "ERROR: push to origin/{branch} rejected for PR #{pr_num}:\n{push_out}"
        ));
        return;
    }
    log(&format!("Pushed Fix Comments commit to origin/{branch}"));

    // Phase 3 (#145): mark every thread we fed into the agent as resolved
    // on GitHub via the resolveReviewThread mutation. The fix is already
    // pushed at this point, so failures here are cosmetic — log them and
    // keep going. Per the #145 acceptance criteria, the run is NOT aborted
    // when one thread fails to resolve.
    log(&format!(
        "Resolving {} addressed thread(s) on PR #{pr_num}...",
        threads.len()
    ));
    let mut resolved = 0u32;
    for t in &threads {
        if resolve_review_thread(&t.id) {
            resolved += 1;
        } else {
            log(&format!(
                "WARNING: could not resolve thread {} ({}:{}) — fix is already pushed, leaving thread open for human review",
                t.id, t.path, t.line
            ));
        }
    }
    log(&format!(
        "Resolved {resolved}/{} thread(s) on PR #{pr_num}",
        threads.len()
    ));

    // _wt_guard drops here, removing the worktree.
}

pub const USAGE: &str = ""; // Legacy, CLI help is now handled by clap

/// Derive a project name from the git repo directory name.
fn infer_project_name(root: &str) -> String {
    std::path::Path::new(root)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into())
}

pub fn parse_args() -> Config {
    let root = cmd_stdout("git", &["rev-parse", "--show-toplevel"])
        .unwrap_or_else(|| die("not inside a git repository"));

    let dev_cfg = crate::agent::types::load_dev_config(&root);
    let bot_settings = load_bot_settings(&root, &dev_cfg);
    let bot_credentials = bot_settings.to_credentials();
    let project_name = env::var("DEV_PROJECT_NAME")
        .ok()
        .or(dev_cfg.project_name)
        .unwrap_or_else(|| infer_project_name(&root));
    let mut local_inference = dev_cfg.local_inference.into_local_inference_config();
    if let Some(api_key) = load_local_inference_api_key(&root) {
        local_inference.api_key = api_key;
    }
    let scan_targets = dev_cfg.security_scan.into_scan_targets();
    let skill_paths = dev_cfg.skills.into_skill_paths();
    let bootstrap_agent_files = dev_cfg.bootstrap_agent_files.unwrap_or(true);

    Config {
        agent: Agent::Claude, // Default, will be overridden by CLI
        auto_mode: false,     // Default, will be overridden by CLI
        dry_run: false,       // Default, will be overridden by CLI
        local_inference,
        root,
        project_name,
        scan_targets,
        skill_paths,
        bootstrap_agent_files,
        workflow_preset: dev_cfg.workflow_preset.unwrap_or_else(|| "default".to_string()),
        bot_settings,
        bot_credentials,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::{LocalInferenceConfig, ScanTargets, SkillPaths};
    use std::fs;

    fn test_config(agent: Agent) -> Config {
        Config {
            agent,
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "freq-cloud".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            workflow_preset: "default".to_string(),
            bot_settings: Default::default(),
            bot_credentials: None,
        }
    }

    fn test_config_at(root: &Path, agent: Agent) -> Config {
        let mut cfg = test_config(agent);
        cfg.root = root.to_string_lossy().into_owned();
        cfg
    }

    /// Exercises the full toak-rs pipeline (`MarkdownGenerator` + `count_tokens`)
    /// inside a tokio runtime to verify `block_in_place` doesn't panic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn toak_generates_snapshot_inside_tokio_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // toak-rs requires a git repo — initialise one with a tracked file.
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()
            .unwrap();
        fs::write(root.join("main.rs"), "fn main() { println!(\"hello\"); }\n").unwrap();
        std::process::Command::new("git")
            .args(["add", "main.rs"])
            .current_dir(root)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root)
            .output()
            .unwrap();

        let snapshot_path = root.join("prompt.md");
        let opts = MarkdownGeneratorOptions {
            dir: root.to_path_buf(),
            output_file_path: snapshot_path.clone(),
            verbose: false,
            ..Default::default()
        };

        let mut generator = MarkdownGenerator::new(opts);

        // This is the exact pattern from generate_codebase_snapshot — panics
        // if block_in_place is missing when called from an async context.
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(generator.create_markdown_document())
        });

        assert!(result.is_ok(), "toak-rs generation failed: {result:?}");
        let res = result.unwrap();
        assert!(res.success, "toak-rs reported failure");

        let content = fs::read_to_string(&snapshot_path).unwrap();
        assert!(!content.is_empty(), "snapshot file should not be empty");
        assert!(
            content.contains("main"),
            "snapshot should contain our source"
        );

        let tokens = count_tokens(&content);
        assert!(
            tokens > 0,
            "count_tokens should return >0 for non-empty input"
        );
    }

    #[test]
    fn truncate_snapshot_under_budget_still_appends_marker() {
        let input = "short".to_string();
        let result = truncate_snapshot(input.clone(), 100);
        assert!(result.starts_with("short"));
        assert!(result.contains("snapshot truncated"));
    }

    #[test]
    fn truncate_snapshot_over_budget_cuts_to_byte_limit() {
        // 10 tokens × 3 bytes/token = 30 bytes max
        let input = "a".repeat(100);
        let result = truncate_snapshot(input, 10);
        let body = result.split("\n\n[...").next().unwrap();
        assert_eq!(body.len(), 30);
    }

    #[test]
    fn truncate_snapshot_respects_char_boundaries() {
        // 'é' is 2 bytes in UTF-8. With max_tokens=5, max_bytes=15.
        // 7 × 'é' = 14 bytes, plus 'a' = 15 bytes exactly. Should not split mid-char.
        let input = "ééééééé".to_string(); // 14 bytes
        let result = truncate_snapshot(input.clone(), 5);
        // max_bytes = 15, input is 14 bytes, so it fits — no truncation of content
        let body = result.split("\n\n[...").next().unwrap();
        assert_eq!(body, "ééééééé");

        // Now force a mid-char split: 3 tokens × 3 = 9 bytes, 'ééééé' = 10 bytes
        let input2 = "ééééé".to_string(); // 10 bytes
        let result2 = truncate_snapshot(input2, 3);
        let body2 = result2.split("\n\n[...").next().unwrap();
        // Should back up to 8 bytes = 4 'é' chars
        assert_eq!(body2, "éééé");
        assert!(body2.len() <= 9);
    }

    #[test]
    fn truncate_snapshot_result_is_within_budget() {
        let input = "fn main() { println!(\"hello world\"); }\n".repeat(10_000);
        let max_tokens = 1_000;
        let result = truncate_snapshot(input, max_tokens);
        let body = result.split("\n\n[...").next().unwrap();
        assert!(body.len() <= max_tokens * 3);
    }

    #[test]
    fn claude_local_inference_overrides_use_anthropic_env_and_model_flag() {
        let mut cfg = test_config(Agent::Claude);
        cfg.local_inference.advanced = true;
        cfg.local_inference.base_url = "http://localhost:8000/v1".into();
        cfg.local_inference.model = "qwen2.5-coder:32b".into();

        let overrides = local_inference_overrides(&cfg);

        assert_eq!(
            overrides.env,
            vec![
                (
                    "ANTHROPIC_BASE_URL".to_string(),
                    "http://localhost:8000/v1".to_string()
                ),
                ("ANTHROPIC_API_KEY".to_string(), "local".to_string()),
            ]
        );
        assert_eq!(
            overrides.args,
            vec!["--model".to_string(), "qwen2.5-coder:32b".to_string()]
        );
    }

    #[test]
    fn codex_local_inference_overrides_use_openai_env_and_config_arg() {
        let mut cfg = test_config(Agent::Codex);
        cfg.local_inference.advanced = true;
        cfg.local_inference.base_url = "http://localhost:1234/v1".into();
        cfg.local_inference.model = "gpt-oss:20b".into();
        cfg.local_inference.api_key = "abc123".into();

        let overrides = local_inference_overrides(&cfg);

        assert_eq!(
            overrides.env,
            vec![
                (
                    "OPENAI_BASE_URL".to_string(),
                    "http://localhost:1234/v1".to_string()
                ),
                ("OPENAI_API_KEY".to_string(), "abc123".to_string()),
            ]
        );
        assert_eq!(
            overrides.args,
            vec![
                "-c".to_string(),
                "openai_base_url=\"http://localhost:1234/v1\"".to_string(),
                "--model".to_string(),
                "gpt-oss:20b".to_string(),
            ]
        );
    }

    #[test]
    fn unsupported_agents_ignore_local_inference_overrides() {
        let mut cfg = test_config(Agent::Gemini);
        cfg.local_inference.advanced = true;
        cfg.local_inference.base_url = "http://localhost:11434/v1".into();

        assert_eq!(
            local_inference_overrides(&cfg),
            AgentLaunchOverrides::default()
        );
    }

    #[test]
    fn api_keys_are_redacted_in_dry_run_logs() {
        assert_eq!(
            redact_launch_env_value("OPENAI_API_KEY", "secret123"),
            "<redacted>"
        );
        assert_eq!(redact_launch_env_value("OPENAI_API_KEY", "local"), "local");
        assert_eq!(
            redact_launch_env_value("OPENAI_BASE_URL", "http://localhost:1234/v1"),
            "http://localhost:1234/v1"
        );
    }

    #[test]
    fn codex_item_agent_message_maps_to_assistant_text() {
        let line = r#"{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"hello"}}"#;
        let events = codex_events_from_json_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Claude(ClaudeEvent::Assistant { message }) => {
                assert_eq!(message.content.len(), 1);
                assert!(matches!(
                    &message.content[0],
                    ContentBlock::Text { text } if text == "hello"
                ));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn codex_command_execution_maps_to_tool_blocks() {
        let started = r#"{"type":"item.started","item":{"id":"item_2","type":"command_execution","command":"echo hi","status":"in_progress"}}"#;
        let completed = r#"{"type":"item.completed","item":{"id":"item_2","type":"command_execution","command":"echo hi","aggregated_output":"hi\n","exit_code":0,"status":"completed"}}"#;

        let started_events = codex_events_from_json_line(started).unwrap();
        let completed_events = codex_events_from_json_line(completed).unwrap();

        assert_eq!(started_events.len(), 1);
        assert_eq!(completed_events.len(), 1);

        match &started_events[0] {
            AgentEvent::Claude(ClaudeEvent::Assistant { message }) => {
                assert!(matches!(
                    &message.content[0],
                    ContentBlock::ToolUse { name, .. } if name == "command_execution"
                ));
            }
            other => panic!("unexpected started event: {other:?}"),
        }

        match &completed_events[0] {
            AgentEvent::Claude(ClaudeEvent::Assistant { message }) => {
                assert!(matches!(
                    &message.content[0],
                    ContentBlock::ToolResult { content, .. } if content.contains("status: completed")
                ));
            }
            other => panic!("unexpected completed event: {other:?}"),
        }
    }

    #[test]
    fn codex_turn_completed_maps_to_result_tokens() {
        let line = r#"{"type":"turn.completed","usage":{"input_tokens":123,"cached_input_tokens":5,"output_tokens":45}}"#;
        let events = codex_events_from_json_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Claude(ClaudeEvent::Result {
                status,
                input_tokens,
                output_tokens,
                ..
            }) => {
                assert_eq!(status, "completed");
                assert_eq!(*input_tokens, Some(123));
                assert_eq!(*output_tokens, Some(45));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn ensure_agent_files_bootstraps_missing_defaults_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let cfg = test_config_at(root, Agent::Codex);

        ensure_agent_files(&cfg);

        assert_eq!(
            fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            AGENTS_MD
        );
        assert!(
            !root.join(".agents/README.md").exists(),
            "bootstrap should not materialize unrelated .agents files"
        );

        for file in SkillAssets::iter() {
            let path = root.join(".agents/skills").join(file.as_ref());
            assert!(
                path.exists(),
                "missing bootstrapped skill: {}",
                path.display()
            );
            let embedded = SkillAssets::get(file.as_ref()).unwrap();
            assert_eq!(fs::read(path).unwrap(), embedded.data.as_ref());
        }
    }

    #[test]
    fn ensure_agent_files_preserves_existing_repo_copies() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let cfg = test_config_at(root, Agent::Codex);

        fs::write(root.join("AGENTS.md"), "custom agents\n").unwrap();
        let testing_skill = root.join(".agents/skills/testing/SKILL.md");
        fs::create_dir_all(testing_skill.parent().unwrap()).unwrap();
        fs::write(&testing_skill, "custom testing skill\n").unwrap();

        ensure_agent_files(&cfg);

        assert_eq!(
            fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            "custom agents\n"
        );
        assert_eq!(
            fs::read_to_string(&testing_skill).unwrap(),
            "custom testing skill\n"
        );
    }

    /// Initialise a temp git repo with an initial commit. Returns the
    /// `TempDir` (must be kept alive) and its path.
    fn init_temp_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let run = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "git {args:?} failed");
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "test"]);
        // Seed an initial commit so HEAD exists.
        fs::write(root.join("README.md"), "initial\n").unwrap();
        fs::write(root.join("STATUS.md"), "initial\n").unwrap();
        run(&["add", "README.md", "STATUS.md"]);
        run(&["commit", "-q", "-m", "init"]);
        dir
    }

    /// #140: scoped status must ignore unrelated dirty worktree state so
    /// the no-op path can exit cleanly when no doc files have actually
    /// drifted.
    #[test]
    fn refresh_docs_scoped_status_ignores_unrelated_dirty_files() {
        let dir = init_temp_repo();
        let root = dir.path();

        // Unrelated working-tree change (not a doc file).
        fs::write(root.join("src.rs"), "fn main() {}\n").unwrap();
        // Unrelated untracked file.
        fs::write(root.join("scratch.txt"), "noise\n").unwrap();

        let doc_files = ["README.md".to_string(), "STATUS.md".to_string()];
        let scoped = git_status_porcelain_scoped(Some(root), &doc_files);
        assert!(
            scoped.trim().is_empty(),
            "scoped status should be empty when no doc files changed; got: {scoped:?}"
        );

        // Sanity: a whole-repo status would NOT be empty here.
        let whole = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(root)
            .output()
            .unwrap();
        let whole_out = String::from_utf8_lossy(&whole.stdout);
        assert!(
            !whole_out.trim().is_empty(),
            "whole-repo status was empty in test fixture: {whole_out:?}"
        );
    }

    /// #140: when a doc file is actually modified, scoped status should
    /// surface it (so the action proceeds to the commit step).
    #[test]
    fn refresh_docs_scoped_status_surfaces_doc_changes() {
        let dir = init_temp_repo();
        let root = dir.path();

        fs::write(root.join("README.md"), "updated\n").unwrap();

        let doc_files = ["README.md".to_string(), "STATUS.md".to_string()];
        let scoped = git_status_porcelain_scoped(Some(root), &doc_files);
        assert!(scoped.contains("README.md"), "got: {scoped:?}");
        assert!(!scoped.contains("STATUS.md"), "got: {scoped:?}");
    }

    /// #139: pre-existing staged out-of-scope files must stay out of the
    /// Refresh Docs commit even after the doc files are staged.
    #[test]
    fn refresh_docs_commit_paths_excludes_preexisting_staged_files() {
        let dir = init_temp_repo();
        let root = dir.path();

        // Pre-existing staged out-of-scope file.
        fs::write(root.join("secret.env"), "TOKEN=abc\n").unwrap();
        let ok = std::process::Command::new("git")
            .args(["add", "secret.env"])
            .current_dir(root)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        assert_eq!(git_staged_files(Some(root)), vec!["secret.env".to_string()]);

        // Simulate the agent producing a doc change.
        fs::write(root.join("README.md"), "agent edit\n").unwrap();
        let ok = std::process::Command::new("git")
            .args(["add", "--", "README.md"])
            .current_dir(root)
            .status()
            .unwrap()
            .success();
        assert!(ok);

        let doc_files = ["README.md".to_string(), "STATUS.md".to_string()];
        assert!(git_commit_paths(
            Some(root),
            "refresh project docs",
            &doc_files
        ));

        let committed = std::process::Command::new("git")
            .args(["show", "--name-only", "--pretty=format:", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(committed.status.success());
        let committed_out = String::from_utf8_lossy(&committed.stdout);
        let staged = git_staged_files(Some(root));
        assert!(
            committed_out.lines().any(|line| line == "README.md"),
            "README.md should be in the docs refresh commit: {committed_out:?}"
        );
        assert!(
            !committed_out.lines().any(|line| line == "secret.env"),
            "secret.env must stay out of the docs refresh commit: {committed_out:?}"
        );
        assert_eq!(staged, vec!["secret.env".to_string()]);
    }

    /// #138: the UXR Synth rename must reach the user-visible dry-run log
    /// strings. This test scans this source file and fails if any leftover
    /// stale-name string literal sneaks back into the file. The needle is
    /// assembled at runtime so this test body itself does not contain the
    /// literal it searches for.
    #[test]
    fn uxr_synth_rename_is_complete_in_log_strings() {
        let src = include_str!("shell.rs");
        let needle_a = format!("\"{} research", "UXR");
        let needle_b = format!("\"{} research", "uxr");
        for (idx, line) in src.lines().enumerate() {
            assert!(
                !line.contains(&needle_a) && !line.contains(&needle_b),
                "shell.rs:{} still contains a stale UXR-rename log string: {line}",
                idx + 1,
            );
        }
    }
}
