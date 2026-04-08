use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;
use tokio::sync::mpsc;

pub static EVENT_SENDER: OnceLock<mpsc::UnboundedSender<AgentEvent>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workflow {
    Ideation,
    ReportResearch,
    StrategicReview,
    Roadmapper,
    SprintPlanning,
    Retrospective,
    Housekeeping,
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Workflow::Ideation => write!(f, "Ideation"),
            Workflow::ReportResearch => write!(f, "UXR Synth"),
            Workflow::StrategicReview => write!(f, "Strategic Review"),
            Workflow::Roadmapper => write!(f, "Roadmapper"),
            Workflow::SprintPlanning => write!(f, "Sprint Planning"),
            Workflow::Retrospective => write!(f, "Retrospective"),
            Workflow::Housekeeping => write!(f, "Housekeeping"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AgentEvent {
    Done,
    Log(String),
    Claude(ClaudeEvent),
    AwaitingFeedback(Workflow),
    TrackerUpdate(Vec<crate::agent::tracker::PendingIssue>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeEvent {
    System {
        subtype: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        claude_code_version: Option<String>,
        #[serde(default)]
        tools: Option<Vec<String>>,
    },
    Assistant {
        message: AssistantMessage,
    },
    User {
        message: UserMessage,
    },
    Result {
        status: String,
        summary: Option<String>,
        #[serde(default)]
        duration_ms: Option<u64>,
        #[serde(default)]
        input_tokens: Option<u32>,
        #[serde(default)]
        output_tokens: Option<u32>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        #[serde(rename = "tool_use_id")]
        id: String,
        content: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Agent {
    Claude,
    Codex,
    Copilot,
    Gemini,
}

impl clap::ValueEnum for Agent {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Claude, Self::Codex, Self::Copilot, Self::Gemini]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Claude => clap::builder::PossibleValue::new("claude"),
            Self::Codex => clap::builder::PossibleValue::new("codex"),
            Self::Copilot => clap::builder::PossibleValue::new("copilot"),
            Self::Gemini => clap::builder::PossibleValue::new("gemini"),
        })
    }
}

impl FromStr for Agent {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Agent::Claude),
            "codex" => Ok(Agent::Codex),
            "copilot" => Ok(Agent::Copilot),
            "gemini" => Ok(Agent::Gemini),
            _ => Err(format!("Unknown agent: {}", s)),
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Agent::Claude => write!(f, "claude"),
            Agent::Codex => write!(f, "codex"),
            Agent::Copilot => write!(f, "copilot"),
            Agent::Gemini => write!(f, "gemini"),
        }
    }
}

impl Agent {
    pub fn binary(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Copilot => "copilot",
            Agent::Gemini => "gemini",
        }
    }

    pub fn co_author(self) -> &'static str {
        match self {
            Agent::Claude => "Co-Authored-By: Claude <noreply@anthropic.com>",
            Agent::Codex => "Co-Authored-By: Codex <noreply@openai.com>",
            Agent::Copilot => "Co-Authored-By: GitHub Copilot <noreply@github.com>",
            Agent::Gemini => "Co-Authored-By: Gemini <noreply@google.com>",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalInferencePreset {
    #[default]
    Vllm,
    LmStudio,
    Ollama,
    Custom,
}

impl LocalInferencePreset {
    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            LocalInferencePreset::Vllm => Some("http://localhost:8000/v1"),
            LocalInferencePreset::LmStudio => Some("http://localhost:1234/v1"),
            LocalInferencePreset::Ollama => Some("http://localhost:11434/v1"),
            LocalInferencePreset::Custom => None,
        }
    }

    pub fn infer_from_base_url(base_url: &str) -> Self {
        for preset in [
            LocalInferencePreset::Vllm,
            LocalInferencePreset::LmStudio,
            LocalInferencePreset::Ollama,
        ] {
            if preset.default_base_url() == Some(base_url) {
                return preset;
            }
        }
        LocalInferencePreset::Custom
    }
}

impl FromStr for LocalInferencePreset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "vllm" => Ok(LocalInferencePreset::Vllm),
            "lm_studio" | "lm-studio" | "lmstudio" => Ok(LocalInferencePreset::LmStudio),
            "ollama" => Ok(LocalInferencePreset::Ollama),
            "custom" => Ok(LocalInferencePreset::Custom),
            _ => Err(format!("Unknown local inference preset: {s}")),
        }
    }
}

impl fmt::Display for LocalInferencePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalInferencePreset::Vllm => write!(f, "vllm"),
            LocalInferencePreset::LmStudio => write!(f, "lm_studio"),
            LocalInferencePreset::Ollama => write!(f, "ollama"),
            LocalInferencePreset::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalInferenceConfig {
    pub advanced: bool,
    pub preset: LocalInferencePreset,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

impl Default for LocalInferenceConfig {
    fn default() -> Self {
        Self {
            advanced: false,
            preset: LocalInferencePreset::Vllm,
            base_url: LocalInferencePreset::Vllm
                .default_base_url()
                .unwrap_or_default()
                .to_string(),
            model: String::new(),
            api_key: String::new(),
        }
    }
}

impl LocalInferenceConfig {
    pub fn apply_preset(&mut self, preset: LocalInferencePreset) {
        self.preset = preset;
        if let Some(base_url) = preset.default_base_url() {
            self.base_url = base_url.to_string();
        }
    }

    pub fn set_base_url(&mut self, base_url: String) {
        self.preset = LocalInferencePreset::infer_from_base_url(base_url.trim());
        self.base_url = base_url;
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotCredentials {
    /// A pre-minted token (PAT or installation token).
    Token(String),
    /// GitHub App credentials for on-demand token minting.
    GitHubApp {
        app_id: String,
        installation_id: String,
        private_key_path: String,
    },
}

/// Debug formatting for `BotCredentials` is fully redacted: serde(skip)
/// only protects the `Serialize` path, but a stray `debug!(?cfg)` or
/// panic-on-Debug path would still expose PAT / GitHub App fields. The
/// manual impl here ensures every Debug rendering is opaque.
impl fmt::Debug for BotCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BotCredentials::Token(_) => f
                .debug_tuple("BotCredentials::Token")
                .field(&"<redacted>")
                .finish(),
            BotCredentials::GitHubApp { .. } => f
                .debug_struct("BotCredentials::GitHubApp")
                .field("app_id", &"<redacted>")
                .field("installation_id", &"<redacted>")
                .field("private_key_path", &"<redacted>")
                .finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub agent: Agent,
    pub auto_mode: bool,
    pub dry_run: bool,
    #[serde(default)]
    pub local_inference: LocalInferenceConfig,
    pub root: String,
    pub project_name: String,
    pub scan_targets: ScanTargets,
    #[serde(default)]
    pub skill_paths: SkillPaths,
    #[serde(default = "default_bootstrap_agent_files")]
    pub bootstrap_agent_files: bool,
    #[serde(skip)]
    pub bot_credentials: Option<BotCredentials>,
}

fn default_bootstrap_agent_files() -> bool {
    true
}

/// Per-skill paths the dev agent reads at runtime. Hardcoded defaults match
/// freq-ai's own `.agents/skills/` layout, so the standalone freq-ai binary
/// keeps working unchanged. Library consumers (e.g. a project that organises
/// its skills under a different prefix) can override these on `Config` before
/// calling `freq_ai::run`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPaths {
    /// Path to the user-personas skill, used by the UXR Synthesis prompt
    /// builders to seed the persona-lens section.
    pub user_personas: String,
    /// Path to the issue-tracking skill, runtime-loaded by the sidebar to
    /// render the "Before marking an issue complete" trigger reminder.
    pub issue_tracking: String,
}

impl Default for SkillPaths {
    fn default() -> Self {
        Self {
            user_personas: ".agents/skills/user-personas/SKILL.md".into(),
            issue_tracking: ".agents/skills/issue-tracking/SKILL.md".into(),
        }
    }
}

/// Debug formatting for `Config` redacts the credential field while
/// leaving the other fields untouched. The variant tag is kept so that
/// debug output still indicates whether credentials were present, but
/// no token or key path can leak.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bot_credentials_marker: &dyn fmt::Debug = match &self.bot_credentials {
            None => &"None",
            Some(BotCredentials::Token(_)) => &"Some(Token(<redacted>))",
            Some(BotCredentials::GitHubApp { .. }) => &"Some(GitHubApp(<redacted>))",
        };
        f.debug_struct("Config")
            .field("agent", &self.agent)
            .field("auto_mode", &self.auto_mode)
            .field("dry_run", &self.dry_run)
            .field("local_inference", &self.local_inference)
            .field("root", &self.root)
            .field("project_name", &self.project_name)
            .field("scan_targets", &self.scan_targets)
            .field("skill_paths", &self.skill_paths)
            .field("bootstrap_agent_files", &self.bootstrap_agent_files)
            .field("bot_credentials", bot_credentials_marker)
            .finish()
    }
}

/// File paths (relative to the project root) that the security scanner inspects.
/// Loaded from `dev.toml` `[security_scan]` section; falls back to sensible defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanTargets {
    pub edge: String,
    pub network_kem: String,
    pub network_crypto: String,
    pub network: String,
    pub service: String,
    pub gateway: String,
    pub gateway_users: String,
    pub gateway_kms: String,
    pub cli_build: String,
    pub compute: String,
}

impl Default for ScanTargets {
    fn default() -> Self {
        Self {
            edge: "crates/edge-node/src/lib.rs".into(),
            network_kem: "crates/network-node/src/kem.rs".into(),
            network_crypto: "crates/network-node/src/crypto.rs".into(),
            network: "crates/network-node/src/lib.rs".into(),
            service: "crates/service-node/src/lib.rs".into(),
            gateway: "crates/gateway-node/src/lib.rs".into(),
            gateway_users: "crates/gateway-node/src/users.rs".into(),
            gateway_kms: "crates/gateway-node/src/kms.rs".into(),
            cli_build: "crates/freq-cli/src/build.rs".into(),
            compute: "crates/compute-node/src/lib.rs".into(),
        }
    }
}

/// On-disk `dev.toml` layout. Missing fields fall back to defaults.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct DevConfig {
    pub project_name: Option<String>,
    #[serde(default)]
    pub local_inference: LocalInferenceConfigFile,
    #[serde(default)]
    pub security_scan: ScanTargetsFile,
    #[serde(default)]
    pub skills: SkillPathsFile,
    /// Whether `preflight()` should materialise embedded default skill files
    /// into the project root if they're missing. Library consumers that bring
    /// their own skill layout (under a different prefix) should set this to
    /// `false` so freq-ai's defaults don't appear next to their own files.
    pub bootstrap_agent_files: Option<bool>,
}

/// Optional overrides for scan target paths in `dev.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ScanTargetsFile {
    pub edge: Option<String>,
    pub network_kem: Option<String>,
    pub network_crypto: Option<String>,
    pub network: Option<String>,
    pub service: Option<String>,
    pub gateway: Option<String>,
    pub gateway_users: Option<String>,
    pub gateway_kms: Option<String>,
    pub cli_build: Option<String>,
    pub compute: Option<String>,
}

/// Optional local inference overrides in `dev.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct LocalInferenceConfigFile {
    pub advanced: Option<bool>,
    pub preset: Option<LocalInferencePreset>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

/// Optional overrides for skill file paths in `dev.toml`'s `[skills]` section.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SkillPathsFile {
    pub user_personas: Option<String>,
    pub issue_tracking: Option<String>,
}

impl SkillPathsFile {
    /// Merge file overrides onto defaults, producing a complete `SkillPaths`.
    pub fn into_skill_paths(self) -> SkillPaths {
        let def = SkillPaths::default();
        SkillPaths {
            user_personas: self.user_personas.unwrap_or(def.user_personas),
            issue_tracking: self.issue_tracking.unwrap_or(def.issue_tracking),
        }
    }
}

impl ScanTargetsFile {
    /// Merge file overrides onto defaults, producing a complete `ScanTargets`.
    pub fn into_scan_targets(self) -> ScanTargets {
        let def = ScanTargets::default();
        ScanTargets {
            edge: self.edge.unwrap_or(def.edge),
            network_kem: self.network_kem.unwrap_or(def.network_kem),
            network_crypto: self.network_crypto.unwrap_or(def.network_crypto),
            network: self.network.unwrap_or(def.network),
            service: self.service.unwrap_or(def.service),
            gateway: self.gateway.unwrap_or(def.gateway),
            gateway_users: self.gateway_users.unwrap_or(def.gateway_users),
            gateway_kms: self.gateway_kms.unwrap_or(def.gateway_kms),
            cli_build: self.cli_build.unwrap_or(def.cli_build),
            compute: self.compute.unwrap_or(def.compute),
        }
    }
}

impl LocalInferenceConfigFile {
    /// Merge file overrides onto defaults, producing a complete local inference config.
    pub fn into_local_inference_config(self) -> LocalInferenceConfig {
        let mut cfg = LocalInferenceConfig::default();

        if let Some(advanced) = self.advanced {
            cfg.advanced = advanced;
        }
        if let Some(preset) = self.preset {
            cfg.apply_preset(preset);
        }
        if let Some(base_url) = self.base_url {
            cfg.set_base_url(base_url);
        }
        if let Some(model) = self.model {
            cfg.model = model;
        }
        if let Some(api_key) = self.api_key {
            cfg.api_key = api_key;
        }

        cfg
    }
}

/// Load `dev.toml` from the project root. Returns defaults if the file is absent or malformed.
pub fn load_dev_config(root: &str) -> DevConfig {
    let path = std::path::Path::new(root).join("dev.toml");
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => DevConfig::default(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FileChangeKind {
    Read,
    Created,
    Modified,
    Deleted,
}

impl std::fmt::Display for FileChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileChangeKind::Read => write!(f, "read"),
            FileChangeKind::Created => write!(f, "created"),
            FileChangeKind::Modified => write!(f, "modified"),
            FileChangeKind::Deleted => write!(f, "deleted"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub kind: FileChangeKind,
}

pub const BRANCH_PREFIX: &str = "agent/issue-";
pub const MAX_COMMIT_ATTEMPTS: u32 = 3;
pub const MAX_PUSH_ATTEMPTS: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    // ── Workflow ──

    #[test]
    fn workflow_display() {
        assert_eq!(Workflow::Ideation.to_string(), "Ideation");
        assert_eq!(Workflow::ReportResearch.to_string(), "UXR Synth");
        assert_eq!(Workflow::StrategicReview.to_string(), "Strategic Review");
        assert_eq!(Workflow::SprintPlanning.to_string(), "Sprint Planning");
        assert_eq!(Workflow::Retrospective.to_string(), "Retrospective");
        assert_eq!(Workflow::Housekeeping.to_string(), "Housekeeping");
    }

    #[test]
    fn workflow_copy_eq() {
        let a = Workflow::Retrospective;
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn workflow_housekeeping_copy_eq() {
        let a = Workflow::Housekeeping;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(a, Workflow::Retrospective);
    }

    // ── Agent parsing ──

    #[test]
    fn agent_from_str_valid() {
        assert_eq!("claude".parse::<Agent>().unwrap(), Agent::Claude);
        assert_eq!("Codex".parse::<Agent>().unwrap(), Agent::Codex);
        assert_eq!("COPILOT".parse::<Agent>().unwrap(), Agent::Copilot);
        assert_eq!("Gemini".parse::<Agent>().unwrap(), Agent::Gemini);
    }

    #[test]
    fn agent_from_str_invalid() {
        assert!("gpt4".parse::<Agent>().is_err());
        assert!("".parse::<Agent>().is_err());
    }

    #[test]
    fn agent_display_roundtrip() {
        for agent in [Agent::Claude, Agent::Codex, Agent::Copilot, Agent::Gemini] {
            let s = agent.to_string();
            assert_eq!(s.parse::<Agent>().unwrap(), agent);
        }
    }

    #[test]
    fn agent_binary_names() {
        assert_eq!(Agent::Claude.binary(), "claude");
        assert_eq!(Agent::Codex.binary(), "codex");
        assert_eq!(Agent::Copilot.binary(), "copilot");
        assert_eq!(Agent::Gemini.binary(), "gemini");
    }

    #[test]
    fn agent_co_author_contains_name() {
        assert!(Agent::Claude.co_author().contains("Claude"));
        assert!(Agent::Codex.co_author().contains("Codex"));
        assert!(Agent::Copilot.co_author().contains("Copilot"));
        assert!(Agent::Gemini.co_author().contains("Gemini"));
    }

    #[test]
    fn local_inference_preset_roundtrip() {
        for preset in [
            LocalInferencePreset::Vllm,
            LocalInferencePreset::LmStudio,
            LocalInferencePreset::Ollama,
            LocalInferencePreset::Custom,
        ] {
            let s = preset.to_string();
            assert_eq!(s.parse::<LocalInferencePreset>().unwrap(), preset);
        }
    }

    #[test]
    fn local_inference_defaults_match_vllm() {
        let cfg = LocalInferenceConfig::default();
        assert!(!cfg.advanced);
        assert_eq!(cfg.preset, LocalInferencePreset::Vllm);
        assert_eq!(cfg.base_url, "http://localhost:8000/v1");
        assert!(cfg.model.is_empty());
        assert!(cfg.api_key.is_empty());
    }

    #[test]
    fn local_inference_apply_preset_prefills_base_url() {
        let mut cfg = LocalInferenceConfig::default();
        cfg.apply_preset(LocalInferencePreset::LmStudio);
        assert_eq!(cfg.preset, LocalInferencePreset::LmStudio);
        assert_eq!(cfg.base_url, "http://localhost:1234/v1");

        cfg.apply_preset(LocalInferencePreset::Custom);
        assert_eq!(cfg.preset, LocalInferencePreset::Custom);
        assert_eq!(cfg.base_url, "http://localhost:1234/v1");
    }

    #[test]
    fn local_inference_base_url_infers_matching_preset() {
        let mut cfg = LocalInferenceConfig::default();
        cfg.set_base_url("http://localhost:11434/v1".into());
        assert_eq!(cfg.preset, LocalInferencePreset::Ollama);

        cfg.set_base_url("http://10.0.0.5:9000/v1".into());
        assert_eq!(cfg.preset, LocalInferencePreset::Custom);
    }

    #[test]
    fn local_inference_file_merges_defaults() {
        let cfg = LocalInferenceConfigFile {
            advanced: Some(true),
            preset: Some(LocalInferencePreset::LmStudio),
            base_url: None,
            model: Some("qwen2.5-coder:32b".into()),
            api_key: None,
        }
        .into_local_inference_config();

        assert!(cfg.advanced);
        assert_eq!(cfg.preset, LocalInferencePreset::LmStudio);
        assert_eq!(cfg.base_url, "http://localhost:1234/v1");
        assert_eq!(cfg.model, "qwen2.5-coder:32b");
        assert!(cfg.api_key.is_empty());
    }

    // ── ClaudeEvent serde ──

    #[test]
    fn claude_system_event_deserialize() {
        let json = r#"{"type":"system","subtype":"init","model":"opus","description":"ready"}"#;
        let ev: ClaudeEvent = serde_json::from_str(json).unwrap();
        match ev {
            ClaudeEvent::System {
                subtype,
                model,
                description,
                ..
            } => {
                assert_eq!(subtype, "init");
                assert_eq!(model.unwrap(), "opus");
                assert_eq!(description.unwrap(), "ready");
            }
            _ => panic!("expected System variant"),
        }
    }

    #[test]
    fn claude_system_event_minimal() {
        let json = r#"{"type":"system","subtype":"init"}"#;
        let ev: ClaudeEvent = serde_json::from_str(json).unwrap();
        match ev {
            ClaudeEvent::System {
                model,
                description,
                session_id,
                claude_code_version,
                tools,
                ..
            } => {
                assert!(model.is_none());
                assert!(description.is_none());
                assert!(session_id.is_none());
                assert!(claude_code_version.is_none());
                assert!(tools.is_none());
            }
            _ => panic!("expected System variant"),
        }
    }

    #[test]
    fn claude_assistant_event_roundtrip() {
        let ev = ClaudeEvent::Assistant {
            message: AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "hello".into(),
                }],
            },
        };
        let json = serde_json::to_string(&ev).unwrap();
        let back: ClaudeEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, back);
    }

    #[test]
    fn claude_result_event_deserialize() {
        let json = r#"{"type":"result","status":"success","summary":"done","duration_ms":1234,"input_tokens":100,"output_tokens":50}"#;
        let ev: ClaudeEvent = serde_json::from_str(json).unwrap();
        match ev {
            ClaudeEvent::Result {
                status,
                summary,
                duration_ms,
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(status, "success");
                assert_eq!(summary.unwrap(), "done");
                assert_eq!(duration_ms.unwrap(), 1234);
                assert_eq!(input_tokens.unwrap(), 100);
                assert_eq!(output_tokens.unwrap(), 50);
            }
            _ => panic!("expected Result variant"),
        }
    }

    #[test]
    fn claude_result_event_minimal() {
        let json = r#"{"type":"result","status":"error"}"#;
        let ev: ClaudeEvent = serde_json::from_str(json).unwrap();
        match ev {
            ClaudeEvent::Result {
                summary,
                duration_ms,
                ..
            } => {
                assert!(summary.is_none());
                assert!(duration_ms.is_none());
            }
            _ => panic!("expected Result variant"),
        }
    }

    // ── ContentBlock serde ──

    #[test]
    fn content_block_text_roundtrip() {
        let block = ContentBlock::Text {
            text: "test".into(),
        };
        let json = serde_json::to_string(&block).unwrap();
        let back: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
    }

    #[test]
    fn content_block_thinking_roundtrip() {
        let block = ContentBlock::Thinking {
            thinking: "hmm".into(),
        };
        let json = serde_json::to_string(&block).unwrap();
        let back: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
    }

    #[test]
    fn content_block_tool_use_roundtrip() {
        let block = ContentBlock::ToolUse {
            id: "t1".into(),
            name: "Read".into(),
            input: serde_json::json!({"path": "/foo"}),
        };
        let json = serde_json::to_string(&block).unwrap();
        let back: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
    }

    #[test]
    fn content_block_tool_result_roundtrip() {
        let block = ContentBlock::ToolResult {
            id: "t1".into(),
            content: "ok".into(),
        };
        let json = serde_json::to_string(&block).unwrap();
        let back: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
    }

    // ── Config serde ──

    #[test]
    fn config_serde_roundtrip() {
        let cfg = Config {
            agent: Agent::Claude,
            auto_mode: true,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bot_credentials: None,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn skill_paths_default_unprefixed_paths() {
        let p = SkillPaths::default();
        assert_eq!(p.user_personas, ".agents/skills/user-personas/SKILL.md");
        assert_eq!(p.issue_tracking, ".agents/skills/issue-tracking/SKILL.md");
    }

    #[test]
    fn skill_paths_file_merges_defaults() {
        let merged = SkillPathsFile {
            user_personas: Some(".agents/skills/freq-cloud-user-personas/SKILL.md".into()),
            issue_tracking: None,
        }
        .into_skill_paths();
        assert_eq!(
            merged.user_personas,
            ".agents/skills/freq-cloud-user-personas/SKILL.md"
        );
        // Falls back to default for the field that wasn't overridden.
        assert_eq!(merged.issue_tracking, ".agents/skills/issue-tracking/SKILL.md");
    }

    #[test]
    fn config_default_bootstrap_is_true_via_serde() {
        // Confirms `default = "default_bootstrap_agent_files"` works: an old
        // dev.toml without bootstrap_agent_files deserializes with the flag on.
        let json = r#"{
            "agent": "Claude",
            "auto_mode": false,
            "dry_run": false,
            "local_inference": {
                "advanced": false,
                "preset": "vllm",
                "base_url": "http://localhost:8000/v1",
                "model": "",
                "api_key": ""
            },
            "root": "/tmp/test",
            "project_name": "x",
            "scan_targets": {
                "edge": "a", "network_kem": "b", "network_crypto": "c",
                "network": "d", "service": "e", "gateway": "f",
                "gateway_users": "g", "gateway_kms": "h",
                "cli_build": "i", "compute": "j"
            }
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert!(cfg.bootstrap_agent_files);
        assert_eq!(cfg.skill_paths, SkillPaths::default());
    }

    #[test]
    fn config_serde_skips_bot_credentials_token() {
        // bot_credentials is #[serde(skip)] so a Config containing a token
        // serializes WITHOUT the token (no leak via logging/debug paths) and
        // deserializes back with bot_credentials = None.
        let cfg = Config {
            agent: Agent::Claude,
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bot_credentials: Some(BotCredentials::Token("ghp_test123".into())),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            !json.contains("ghp_test123"),
            "bot token must not appear in serialized Config"
        );
        let back: Config = serde_json::from_str(&json).unwrap();
        assert!(back.bot_credentials.is_none());
    }

    /// #136: `#[serde(skip)]` only blocks the serde path. The Debug impl
    /// must also redact bot credentials so a stray `debug!(?cfg)` or
    /// panic-on-Debug path cannot expose PAT / GitHub App fields.
    #[test]
    fn config_debug_redacts_bot_credentials_token() {
        let cfg = Config {
            agent: Agent::Claude,
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bot_credentials: Some(BotCredentials::Token("ghp_test123".into())),
        };
        let dbg = format!("{cfg:?}");
        assert!(
            !dbg.contains("ghp_test123"),
            "bot token leaked into Debug output: {dbg}"
        );
        // The debug output should still surface the variant so reviewers
        // can tell credentials were configured at all.
        assert!(dbg.contains("Token"), "expected token marker in: {dbg}");
        // Other fields must still render (sanity).
        assert!(dbg.contains("my-project"));
    }

    #[test]
    fn config_debug_redacts_bot_credentials_github_app() {
        let cfg = Config {
            agent: Agent::Claude,
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bot_credentials: Some(BotCredentials::GitHubApp {
                app_id: "12345".into(),
                installation_id: "67890".into(),
                private_key_path: "/tmp/key.pem".into(),
            }),
        };
        let dbg = format!("{cfg:?}");
        assert!(
            !dbg.contains("12345") && !dbg.contains("67890") && !dbg.contains("/tmp/key.pem"),
            "GitHub App credentials leaked into Debug output: {dbg}"
        );
        assert!(
            dbg.contains("GitHubApp"),
            "expected GitHubApp marker in: {dbg}"
        );
    }

    #[test]
    fn bot_credentials_debug_is_redacted_directly() {
        let token = BotCredentials::Token("super-secret-pat".into());
        let dbg = format!("{token:?}");
        assert!(!dbg.contains("super-secret-pat"));
        assert!(dbg.contains("redacted"));

        let app = BotCredentials::GitHubApp {
            app_id: "appid42".into(),
            installation_id: "instid99".into(),
            private_key_path: "/secret/key.pem".into(),
        };
        let dbg = format!("{app:?}");
        assert!(!dbg.contains("appid42"));
        assert!(!dbg.contains("instid99"));
        assert!(!dbg.contains("/secret/key.pem"));
        assert!(dbg.contains("redacted"));
    }

    #[test]
    fn config_serde_skips_bot_credentials_github_app() {
        let cfg = Config {
            agent: Agent::Claude,
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bot_credentials: Some(BotCredentials::GitHubApp {
                app_id: "12345".into(),
                installation_id: "67890".into(),
                private_key_path: "/tmp/key.pem".into(),
            }),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            !json.contains("/tmp/key.pem") && !json.contains("12345"),
            "GitHub App credentials must not appear in serialized Config"
        );
        let back: Config = serde_json::from_str(&json).unwrap();
        assert!(back.bot_credentials.is_none());
    }

    // ── AgentEvent variants ──

    #[test]
    fn agent_event_done_eq() {
        assert_eq!(AgentEvent::Done, AgentEvent::Done);
    }

    #[test]
    fn agent_event_log_eq() {
        assert_eq!(AgentEvent::Log("hi".into()), AgentEvent::Log("hi".into()));
        assert_ne!(AgentEvent::Log("hi".into()), AgentEvent::Log("bye".into()));
    }

    #[test]
    fn agent_event_awaiting_feedback_eq() {
        assert_eq!(
            AgentEvent::AwaitingFeedback(Workflow::Retrospective),
            AgentEvent::AwaitingFeedback(Workflow::Retrospective)
        );
        assert_ne!(
            AgentEvent::AwaitingFeedback(Workflow::Retrospective),
            AgentEvent::AwaitingFeedback(Workflow::SprintPlanning)
        );
    }

    #[test]
    fn agent_event_variants_not_equal() {
        assert_ne!(AgentEvent::Done, AgentEvent::Log("done".into()));
    }
}
