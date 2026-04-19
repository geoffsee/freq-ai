use crate::agent::cmd::log;
use crate::agent::launch::{auto_mode_overrides, merged_agent_env, model_selection_overrides};
use crate::agent::process::{emit_event, set_active_child_pid, stop_requested};
use crate::agent::types::{Agent, AgentEvent, AssistantMessage, ClaudeEvent, Config, ContentBlock};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_claude_native_with_env(
    binary: &str,
    args: &[String],
    extra_env: &[(String, String)],
    cwd: Option<&Path>,
) -> bool {
    let mut cmd = Command::new(binary);
    cmd.args(args);

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

pub fn u64_to_u32(value: Option<u64>) -> Option<u32> {
    value.and_then(|v| u32::try_from(v).ok())
}

pub fn assistant_text_event(text: String) -> AgentEvent {
    AgentEvent::Claude(ClaudeEvent::Assistant {
        message: AssistantMessage {
            content: vec![ContentBlock::Text { text }],
        },
    })
}

pub fn assistant_block_event(block: ContentBlock) -> AgentEvent {
    AgentEvent::Claude(ClaudeEvent::Assistant {
        message: AssistantMessage {
            content: vec![block],
        },
    })
}

pub fn codex_events_from_json_line(line: &str) -> Option<Vec<AgentEvent>> {
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
            let _item_id = item
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("codex_item")
                .to_string();
            let item_type = item
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            match item_type {
                "message" => {
                    if let Some(content_arr) =
                        item.get("content").and_then(serde_json::Value::as_array)
                    {
                        for c in content_arr {
                            if let Some(text) = c.get("text").and_then(serde_json::Value::as_str)
                                && !is_completed
                            {
                                out.push(assistant_text_event(text.to_string()));
                            }
                        }
                    }
                }
                "tool_call" => {
                    if let Some(call) = item.get("call").and_then(serde_json::Value::as_object) {
                        let name = call
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("");
                        let args = call
                            .get("arguments")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("");
                        if !is_completed {
                            out.push(assistant_block_event(ContentBlock::ToolUse {
                                id: "codex_tool".to_string(),
                                name: name.to_string(),
                                input: serde_json::from_str(args).unwrap_or(serde_json::json!({})),
                            }));
                        }
                    }
                }
                _ => {}
            }
        }
        "delta.started" => {
            if let Some(delta) = v.get("delta").and_then(serde_json::Value::as_object)
                && let Some(text) = delta.get("text").and_then(serde_json::Value::as_str)
            {
                out.push(AgentEvent::Claude(ClaudeEvent::ContentBlockDelta {
                    index: 0,
                    delta: crate::agent::types::ContentBlockDelta {
                        delta_type: "text_delta".to_string(),
                        text: Some(text.to_string()),
                    },
                }));
            }
        }
        _ => {}
    }
    Some(out)
}

pub fn run_codex_native_with_env(
    binary: &str,
    args: &[String],
    extra_env: &[(String, String)],
    cwd: Option<&Path>,
) -> bool {
    let mut cmd = Command::new(binary);
    cmd.args(args);

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

pub fn run_agent(cfg: &Config, prompt: &str) -> bool {
    run_agent_with_env(cfg, prompt, &[])
}

pub fn run_agent_with_env(cfg: &Config, prompt: &str, extra_env: &[(String, String)]) -> bool {
    let env = merged_agent_env(cfg, extra_env);
    let mut overrides = local_inference_overrides(cfg);
    let model_ov = model_selection_overrides(cfg);
    overrides.args.extend(model_ov.args);
    let auto_ov = auto_mode_overrides(cfg);
    overrides.args.extend(auto_ov.args);

    match cfg.agent {
        Agent::Claude | Agent::Junie | Agent::Copilot | Agent::Cursor => {
            let binary = match cfg.agent {
                Agent::Claude => "claude",
                Agent::Junie => "junie",
                Agent::Copilot => "copilot",
                Agent::Cursor => "cursor",
                _ => unreachable!(),
            };
            let mut args = vec![
                "-p".to_string(),
                prompt.to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
            ];
            args.extend(overrides.args);
            run_claude_native_with_env(binary, &args, &env, None)
        }
        Agent::Codex => {
            let mut args = vec!["chat".to_string(), prompt.to_string()];
            args.extend(overrides.args);
            run_codex_native_with_env("codex", &args, &env, None)
        }
        Agent::Gemini | Agent::Grok => {
            let binary = match cfg.agent {
                Agent::Gemini => "gemini",
                Agent::Grok => "grok",
                _ => unreachable!(),
            };
            let mut args = vec!["-p".to_string(), prompt.to_string()];
            args.extend(overrides.args);
            run_claude_native_with_env(binary, &args, &env, None)
        }
        Agent::Xai => {
            let mut args = vec!["-p".to_string(), prompt.to_string()];
            args.extend(overrides.args);
            run_claude_native_with_env("copilot", &args, &env, None)
        }
        Agent::Cline => {
            let mut args = vec!["chat".to_string(), prompt.to_string()];
            args.extend(overrides.args);
            run_claude_native_with_env("cline", &args, &env, None)
        }
    }
}

pub fn local_inference_overrides(cfg: &Config) -> crate::agent::types::AgentLaunchOverrides {
    crate::agent::launch::local_inference_overrides(cfg)
}
