use crate::agent::cmd::log;
use crate::agent::types::{Agent, AgentLaunchOverrides, Config};

fn local_inference_api_key(api_key: &str) -> String {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        "local".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn local_inference_overrides(cfg: &Config) -> AgentLaunchOverrides {
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
        _ => {
            return AgentLaunchOverrides::default();
        }
    }

    if !model.is_empty() {
        overrides
            .args
            .extend(["--model".to_string(), model.to_string()]);
    }

    overrides
}

/// Generate CLI arguments and/or env vars to pass the user's model selection
/// to the agent subprocess. Returns empty overrides when the model is "Default"
/// (empty string) or when local_inference already provides a model override.
pub fn model_selection_overrides(cfg: &Config) -> AgentLaunchOverrides {
    // Local inference model takes priority.
    if cfg.local_inference.advanced
        && !cfg.local_inference.base_url.trim().is_empty()
        && !cfg.local_inference.model.trim().is_empty()
    {
        return AgentLaunchOverrides::default();
    }

    let model = cfg.model.trim();
    if model.is_empty() {
        return AgentLaunchOverrides::default();
    }

    let mut overrides = AgentLaunchOverrides::default();

    match cfg.agent {
        Agent::Claude | Agent::Junie | Agent::Copilot | Agent::Cursor => {
            overrides.args.extend(["--model".into(), model.into()]);
        }
        Agent::Codex => {
            overrides
                .args
                .extend(["-c".into(), format!("model={model:?}")]);
        }
        Agent::Gemini | Agent::Grok => {
            overrides.args.extend(["-m".into(), model.into()]);
        }
        Agent::Xai => {
            overrides.env.push(("COPILOT_MODEL".into(), model.into()));
        }
        Agent::Cline => {
            // No runtime model flag; configured via `cline auth`.
        }
    }

    overrides
}

/// Generate CLI arguments to skip permission prompts when `auto_mode` is enabled.
/// Returns empty overrides when `auto_mode` is false or the agent has no such flag.
pub fn auto_mode_overrides(cfg: &Config) -> AgentLaunchOverrides {
    if !cfg.auto_mode {
        return AgentLaunchOverrides::default();
    }

    let mut overrides = AgentLaunchOverrides::default();

    match cfg.agent {
        Agent::Claude => {
            overrides.args.push("--dangerously-skip-permissions".into());
        }
        Agent::Codex => {
            overrides
                .args
                .push("--dangerously-bypass-approvals-and-sandbox".into());
        }
        Agent::Cline => {
            overrides.args.push("--yolo".into());
        }
        Agent::Gemini | Agent::Xai | Agent::Cursor => {
            overrides.args.push("--yolo".into());
        }
        Agent::Grok => {
            overrides.args.push("--sandbox".into());
        }
        Agent::Junie => {
            overrides.args.push("--brave".into());
        }
        Agent::Copilot => {
            overrides.args.push("--yolo".into());
        }
    }

    overrides
}

pub fn merged_agent_env(cfg: &Config, extra_env: &[(String, String)]) -> Vec<(String, String)> {
    let mut env = local_inference_overrides(cfg).env;
    env.extend(model_selection_overrides(cfg).env);
    env.extend(extra_env.iter().cloned());

    // #11: Ensure `gh` never uses a pager (e.g. `less`) when the agent
    // invokes it to fetch context or submit PRs.
    env.push(("GH_PAGER".to_string(), "cat".to_string()));

    if cfg.use_subscription {
        env.push(("ANTHROPIC_API_KEY".to_string(), "".to_string()));
        env.push(("OPENAI_API_KEY".to_string(), "".to_string()));
        env.push(("GEMINI_API_KEY".to_string(), "".to_string()));
        env.push(("GROK_API_KEY".to_string(), "".to_string()));
        env.push(("JUNIE_API_KEY".to_string(), "".to_string()));
        env.push(("XAI_API_KEY".to_string(), "".to_string()));
    }

    env
}

fn redact_launch_env_value(key: &str, value: &str) -> String {
    if key.ends_with("API_KEY") && !value.is_empty() && value != "local" {
        "<redacted>".to_string()
    } else {
        value.to_string()
    }
}

pub fn log_resolved_agent_launch(cfg: &Config, extra_env: &[(String, String)]) {
    let mut overrides = local_inference_overrides(cfg);
    let model_ov = model_selection_overrides(cfg);
    overrides.args.extend(model_ov.args);
    let auto_ov = auto_mode_overrides(cfg);
    overrides.args.extend(auto_ov.args);
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
