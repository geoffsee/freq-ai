//! Dispatches [`cli_common::Agent`] to provider [`agent_common::AgentCliAdapter`] implementations.
//! All binary names and flag spellings for subprocess construction live in the provider crates.

use agent_common::{AgentCliAdapter, AgentCliCommand};
use claude::{ClaudeWrapper, CursorWrapper};
use cli_common::Agent;
use cline::ClineWrapper;
use codex::CodexWrapper;
use copilot::CopilotWrapper;
use gemini::GeminiWrapper;
use grok::GrokWrapper;
use junie::JunieWrapper;
use xai::XaiWrapper;

pub fn native_base_command(agent: Agent, prompt: &str) -> AgentCliCommand {
    match agent {
        Agent::Claude => AgentCliCommand {
            binary: ClaudeWrapper.binary().to_string(),
            args: ClaudeWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Cursor => AgentCliCommand {
            binary: CursorWrapper.binary().to_string(),
            args: CursorWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Junie => AgentCliCommand {
            binary: JunieWrapper.binary().to_string(),
            args: JunieWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Copilot => AgentCliCommand {
            binary: CopilotWrapper.binary().to_string(),
            args: CopilotWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Codex => AgentCliCommand {
            binary: CodexWrapper.binary().to_string(),
            args: CodexWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Gemini => AgentCliCommand {
            binary: GeminiWrapper.binary().to_string(),
            args: GeminiWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Grok => AgentCliCommand {
            binary: GrokWrapper.binary().to_string(),
            args: GrokWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Xai => AgentCliCommand {
            binary: XaiWrapper.binary().to_string(),
            args: XaiWrapper.freqai_native_run_argv(prompt),
        },
        Agent::Cline => AgentCliCommand {
            binary: ClineWrapper.binary().to_string(),
            args: ClineWrapper.freqai_native_run_argv(prompt),
        },
    }
}

pub fn freqai_native_command(agent: Agent, prompt: &str, extra_args: &[String]) -> AgentCliCommand {
    let mut cmd = native_base_command(agent, prompt);
    cmd.args.extend_from_slice(extra_args);
    cmd
}

pub fn launch_model_selection(agent: Agent, model: &str) -> (Vec<String>, Vec<(String, String)>) {
    match agent {
        Agent::Claude => ClaudeWrapper.launch_model_selection(model),
        Agent::Cursor => CursorWrapper.launch_model_selection(model),
        Agent::Junie => JunieWrapper.launch_model_selection(model),
        Agent::Copilot => CopilotWrapper.launch_model_selection(model),
        Agent::Codex => CodexWrapper.launch_model_selection(model),
        Agent::Gemini => GeminiWrapper.launch_model_selection(model),
        Agent::Grok => GrokWrapper.launch_model_selection(model),
        Agent::Xai => XaiWrapper.launch_model_selection(model),
        Agent::Cline => ClineWrapper.launch_model_selection(model),
    }
}

pub fn launch_auto_mode(agent: Agent) -> Vec<String> {
    match agent {
        Agent::Claude => ClaudeWrapper.launch_auto_mode(),
        Agent::Cursor => CursorWrapper.launch_auto_mode(),
        Agent::Junie => JunieWrapper.launch_auto_mode(),
        Agent::Copilot => CopilotWrapper.launch_auto_mode(),
        Agent::Codex => CodexWrapper.launch_auto_mode(),
        Agent::Gemini => GeminiWrapper.launch_auto_mode(),
        Agent::Grok => GrokWrapper.launch_auto_mode(),
        Agent::Xai => XaiWrapper.launch_auto_mode(),
        Agent::Cline => ClineWrapper.launch_auto_mode(),
    }
}

pub fn launch_local_inference(
    agent: Agent,
    base_url: &str,
    api_key: &str,
    local_model: &str,
) -> (Vec<String>, Vec<(String, String)>) {
    match agent {
        Agent::Claude => ClaudeWrapper.launch_local_inference(base_url, api_key, local_model),
        Agent::Cursor => CursorWrapper.launch_local_inference(base_url, api_key, local_model),
        Agent::Codex => CodexWrapper.launch_local_inference(base_url, api_key, local_model),
        _ => (Vec::new(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_common::claude_family_native_argv;
    use cli_common::Agent;

    #[test]
    fn native_base_matches_claude_family_and_distinct_agents() {
        let p = "do the thing";
        assert_eq!(native_base_command(Agent::Cursor, p).binary, "cursor");
        assert_eq!(
            native_base_command(Agent::Claude, p).args,
            claude_family_native_argv(p)
        );
        assert_eq!(
            native_base_command(Agent::Junie, p).args,
            claude_family_native_argv(p)
        );
        assert_eq!(
            native_base_command(Agent::Cursor, p).args,
            claude_family_native_argv(p)
        );
        assert_eq!(
            native_base_command(Agent::Codex, p).args,
            vec!["exec".to_string(), "--json".to_string(), p.to_string()]
        );
        assert_eq!(
            native_base_command(Agent::Cline, p).args,
            vec!["chat".to_string(), p.to_string()]
        );
        assert_eq!(
            native_base_command(Agent::Copilot, p).args,
            vec!["-p".to_string(), p.to_string()]
        );
    }

    #[test]
    fn freqai_native_command_appends_overrides_after_base() {
        let extra = vec!["--model".to_string(), "m".to_string()];
        let cmd = freqai_native_command(Agent::Gemini, "hi", &extra);
        assert_eq!(cmd.args[0..2], ["-p", "hi"]);
        assert_eq!(cmd.args[2..], ["--model", "m"]);
    }

    #[test]
    fn xai_model_selection_uses_env_not_args() {
        let (args, env) = launch_model_selection(Agent::Xai, "grok-3");
        assert!(args.is_empty());
        assert_eq!(
            env,
            vec![("COPILOT_MODEL".to_string(), "grok-3".to_string())]
        );
    }
}
