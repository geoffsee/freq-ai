use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    SprintPoker,
    PreIpm,
    Ipm,
    Retrospective,
    Housekeeping,
    Interview,
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Workflow::Ideation => write!(f, "Ideation"),
            Workflow::ReportResearch => write!(f, "UXR Synth"),
            Workflow::StrategicReview => write!(f, "Strategic Review"),
            Workflow::Roadmapper => write!(f, "Roadmapper"),
            Workflow::SprintPlanning => write!(f, "Sprint Planning"),
            Workflow::SprintPoker => write!(f, "Sprint Poker"),
            Workflow::PreIpm => write!(f, "Pre-IPM"),
            Workflow::Ipm => write!(f, "IPM"),
            Workflow::Retrospective => write!(f, "Retrospective"),
            Workflow::Housekeeping => write!(f, "Housekeeping"),
            Workflow::Interview => write!(f, "Interview"),
        }
    }
}

impl Workflow {
    /// Map a workflow YAML `id` string to the corresponding enum variant.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "ideation" => Some(Self::Ideation),
            "report_research" => Some(Self::ReportResearch),
            "strategic_review" => Some(Self::StrategicReview),
            "roadmapper" => Some(Self::Roadmapper),
            "sprint_planning" => Some(Self::SprintPlanning),
            "sprint_poker" => Some(Self::SprintPoker),
            "pre_ipm" => Some(Self::PreIpm),
            "ipm" => Some(Self::Ipm),
            "retrospective" => Some(Self::Retrospective),
            "housekeeping" => Some(Self::Housekeeping),
            "interview" => Some(Self::Interview),
            _ => None,
        }
    }

    /// Return the YAML `id` string for this workflow variant.
    pub fn to_id(&self) -> &'static str {
        match self {
            Self::Ideation => "ideation",
            Self::ReportResearch => "report_research",
            Self::StrategicReview => "strategic_review",
            Self::Roadmapper => "roadmapper",
            Self::SprintPlanning => "sprint_planning",
            Self::SprintPoker => "sprint_poker",
            Self::PreIpm => "pre_ipm",
            Self::Ipm => "ipm",
            Self::Retrospective => "retrospective",
            Self::Housekeeping => "housekeeping",
            Self::Interview => "interview",
        }
    }
}

/// A single turn in the interview dialog, used by the UI to render the
/// conversation view.
#[derive(Clone, Debug, PartialEq)]
pub struct InterviewTurn {
    /// `true` = agent message, `false` = user message.
    pub is_agent: bool,
    pub content: String,
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
    Cline,
    Codex,
    Copilot,
    Gemini,
    Grok,
    Junie,
    Xai,
}

impl clap::ValueEnum for Agent {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Claude,
            Self::Cline,
            Self::Codex,
            Self::Copilot,
            Self::Gemini,
            Self::Grok,
            Self::Junie,
            Self::Xai,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Claude => clap::builder::PossibleValue::new("claude"),
            Self::Cline => clap::builder::PossibleValue::new("cline"),
            Self::Codex => clap::builder::PossibleValue::new("codex"),
            Self::Copilot => clap::builder::PossibleValue::new("copilot"),
            Self::Gemini => clap::builder::PossibleValue::new("gemini"),
            Self::Grok => clap::builder::PossibleValue::new("grok"),
            Self::Junie => clap::builder::PossibleValue::new("junie"),
            Self::Xai => clap::builder::PossibleValue::new("xai"),
        })
    }
}

impl FromStr for Agent {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Agent::Claude),
            "cline" => Ok(Agent::Cline),
            "codex" => Ok(Agent::Codex),
            "copilot" => Ok(Agent::Copilot),
            "gemini" => Ok(Agent::Gemini),
            "grok" => Ok(Agent::Grok),
            "junie" => Ok(Agent::Junie),
            "xai" => Ok(Agent::Xai),
            _ => Err(format!("Unknown agent: {}", s)),
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Agent::Claude => write!(f, "claude"),
            Agent::Cline => write!(f, "cline"),
            Agent::Codex => write!(f, "codex"),
            Agent::Copilot => write!(f, "copilot"),
            Agent::Gemini => write!(f, "gemini"),
            Agent::Grok => write!(f, "grok"),
            Agent::Junie => write!(f, "junie"),
            Agent::Xai => write!(f, "xai"),
        }
    }
}

impl Agent {
    pub fn binary(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Cline => "cline",
            Agent::Codex => "codex",
            Agent::Copilot => "copilot",
            Agent::Gemini => "gemini",
            Agent::Grok => "grok",
            Agent::Junie => "junie",
            Agent::Xai => "copilot", // xAI proxies the copilot CLI
        }
    }

    pub fn co_author(self) -> &'static str {
        match self {
            Agent::Claude => "Co-Authored-By: Claude <noreply@anthropic.com>",
            Agent::Cline => "Co-Authored-By: Cline <noreply@cline.bot>",
            Agent::Codex => "Co-Authored-By: Codex <noreply@openai.com>",
            Agent::Copilot => "Co-Authored-By: GitHub Copilot <noreply@github.com>",
            Agent::Gemini => "Co-Authored-By: Gemini <noreply@google.com>",
            Agent::Grok => "Co-Authored-By: Grok <noreply@x.ai>",
            Agent::Junie => "Co-Authored-By: Junie <junie@jetbrains.com>",
            Agent::Xai => "Co-Authored-By: xAI <noreply@x.ai>",
        }
    }

    /// Models available for selection in the UI dropdown.
    /// Each entry is (model_id, display_label). Empty slice means the agent
    /// has no model flag and the dropdown should be hidden.
    pub fn available_models(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Agent::Claude => &[
                ("claude-sonnet-4-6", "Sonnet 4.6"),
                ("claude-opus-4-6", "Opus 4.6"),
                ("claude-haiku-4-5-20251001", "Haiku 4.5"),
            ],
            Agent::Cline => &[], // No runtime model flag
            Agent::Codex => &[
                ("o3", "o3"),
                ("o4-mini", "o4-mini"),
                ("gpt-4.1", "GPT-4.1"),
                ("gpt-4.1-mini", "GPT-4.1 Mini"),
            ],
            Agent::Copilot => &[
                ("gpt-5.2", "GPT-5.2"),
                ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
                ("claude-opus-4-6", "Claude Opus 4.6"),
            ],
            Agent::Gemini => &[
                ("gemini-2.5-pro", "Gemini 2.5 Pro"),
                ("gemini-2.5-flash", "Gemini 2.5 Flash"),
                ("gemini-2.0-flash", "Gemini 2.0 Flash"),
            ],
            Agent::Grok => &[
                ("grok-4-latest", "Grok 4"),
                ("grok-4-1-fast-reasoning", "Grok 4.1 Fast"),
                ("grok-code-fast-1", "Grok Code Fast"),
                ("grok-3", "Grok 3"),
            ],
            Agent::Junie => &[
                ("claude-sonnet-4-6", "Sonnet 4.6"),
                ("claude-opus-4-6", "Opus 4.6"),
            ],
            Agent::Xai => &[
                ("grok-4-1-fast-reasoning", "Grok 4.1 Fast"),
                ("grok-3", "Grok 3"),
                ("grok-3-mini", "Grok 3 Mini"),
            ],
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotAuthMode {
    #[default]
    Disabled,
    Token,
    #[serde(rename = "github_app")]
    GitHubApp,
}

impl fmt::Display for BotAuthMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BotAuthMode::Disabled => write!(f, "disabled"),
            BotAuthMode::Token => write!(f, "token"),
            BotAuthMode::GitHubApp => write!(f, "github_app"),
        }
    }
}

impl FromStr for BotAuthMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "disabled" | "none" => Ok(BotAuthMode::Disabled),
            "token" => Ok(BotAuthMode::Token),
            "github_app" | "github-app" | "githubapp" => Ok(BotAuthMode::GitHubApp),
            _ => Err(format!("Unknown bot auth mode: {s}")),
        }
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
        private_key_pem: String,
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
                .field("private_key_pem", &"<redacted>")
                .finish(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BotSettings {
    pub mode: BotAuthMode,
    pub token: String,
    pub app_id: String,
    pub installation_id: String,
    pub private_key_pem: String,
}

impl BotSettings {
    pub fn from_credentials(creds: &BotCredentials) -> Self {
        match creds {
            BotCredentials::Token(token) => Self {
                mode: BotAuthMode::Token,
                token: token.clone(),
                ..Self::default()
            },
            BotCredentials::GitHubApp {
                app_id,
                installation_id,
                private_key_pem,
            } => Self {
                mode: BotAuthMode::GitHubApp,
                app_id: app_id.clone(),
                installation_id: installation_id.clone(),
                private_key_pem: private_key_pem.clone(),
                ..Self::default()
            },
        }
    }

    pub fn to_credentials(&self) -> Option<BotCredentials> {
        match self.mode {
            BotAuthMode::Disabled => None,
            BotAuthMode::Token => {
                let token = self.token.trim();
                if token.is_empty() {
                    None
                } else {
                    Some(BotCredentials::Token(token.to_string()))
                }
            }
            BotAuthMode::GitHubApp => {
                let app_id = self.app_id.trim();
                let installation_id = self.installation_id.trim();
                let private_key_pem = self.private_key_pem.trim();
                if app_id.is_empty() || installation_id.is_empty() || private_key_pem.is_empty() {
                    None
                } else {
                    Some(BotCredentials::GitHubApp {
                        app_id: app_id.to_string(),
                        installation_id: installation_id.to_string(),
                        private_key_pem: private_key_pem.to_string(),
                    })
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub agent: Agent,
    #[serde(default)]
    pub model: String,
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
    #[serde(default = "default_true")]
    pub bootstrap_snapshot: bool,
    #[serde(default = "default_workflow_preset")]
    pub workflow_preset: String,
    #[serde(skip)]
    pub bot_settings: BotSettings,
    #[serde(skip)]
    pub bot_credentials: Option<BotCredentials>,
}

impl Config {
    pub fn effective_bot_credentials(&self) -> Option<BotCredentials> {
        self.bot_settings
            .to_credentials()
            .or_else(|| self.bot_credentials.clone())
    }

    pub fn has_bot_credentials(&self) -> bool {
        self.effective_bot_credentials().is_some()
    }
}

fn default_bootstrap_agent_files() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_workflow_preset() -> String {
    "default".to_string()
}

fn is_none<T>(value: &Option<T>) -> bool {
    value.is_none()
}

fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}

/// Per-skill paths the dev agent reads at runtime. Defaults point to the
/// app-data directory (`~/.local/share/freq-ai/skills/`) where embedded
/// assets are materialized, so the target repo is never mutated. Library
/// consumers can override these on `Config` before calling `freq_ai::run`.
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
        let base = crate::agent::assets::assets_dir().join("skills");
        Self {
            user_personas: base.join("user-personas/SKILL.md").to_string_lossy().into(),
            issue_tracking: base
                .join("issue-tracking/SKILL.md")
                .to_string_lossy()
                .into(),
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
            .field("model", &self.model)
            .field("auto_mode", &self.auto_mode)
            .field("dry_run", &self.dry_run)
            .field("local_inference", &self.local_inference)
            .field("root", &self.root)
            .field("project_name", &self.project_name)
            .field("scan_targets", &self.scan_targets)
            .field("skill_paths", &self.skill_paths)
            .field("bootstrap_agent_files", &self.bootstrap_agent_files)
            .field("bot_settings", &format_args!("{}", self.bot_settings.mode))
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
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DevConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub local_inference: LocalInferenceConfigFile,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub security_scan: ScanTargetsFile,
    #[serde(default, skip_serializing_if = "is_default")]
    pub skills: SkillPathsFile,
    /// Whether `preflight()` should materialise embedded default skill files
    /// into the project root if they're missing. Library consumers that bring
    /// their own skill layout (under a different prefix) should set this to
    /// `false` so freq-ai's defaults don't appear next to their own files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_agent_files: Option<bool>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub bot: BotSettingsFile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_snapshot: Option<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agent_models: HashMap<String, String>,
}

/// Optional overrides for scan target paths in `dev.toml`.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct ScanTargetsFile {
    #[serde(skip_serializing_if = "is_none")]
    pub edge: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub network_kem: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub network_crypto: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub gateway_users: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub gateway_kms: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub cli_build: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub compute: Option<String>,
}

/// Optional local inference overrides in `dev.toml`.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct LocalInferenceConfigFile {
    #[serde(skip_serializing_if = "is_none")]
    pub advanced: Option<bool>,
    #[serde(skip_serializing_if = "is_none")]
    pub preset: Option<LocalInferencePreset>,
    #[serde(skip_serializing_if = "is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Optional overrides for skill file paths in `dev.toml`'s `[skills]` section.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct SkillPathsFile {
    #[serde(skip_serializing_if = "is_none")]
    pub user_personas: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub issue_tracking: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct BotSettingsFile {
    #[serde(skip_serializing_if = "is_none")]
    pub mode: Option<BotAuthMode>,
    #[serde(skip_serializing_if = "is_none")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub installation_id: Option<String>,
}

impl BotSettingsFile {
    pub fn into_bot_settings(self) -> BotSettings {
        BotSettings {
            mode: self.mode.unwrap_or_default(),
            token: String::new(),
            app_id: self.app_id.unwrap_or_default(),
            installation_id: self.installation_id.unwrap_or_default(),
            private_key_pem: String::new(),
        }
    }
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

pub fn save_dev_config(root: &str, cfg: &Config) -> Result<(), String> {
    let path = std::path::Path::new(root).join("dev.toml");

    // Merge current model selection into the persisted per-agent map.
    let mut agent_models = load_dev_config(root).agent_models;
    if cfg.model.trim().is_empty() {
        agent_models.remove(&cfg.agent.to_string());
    } else {
        agent_models.insert(cfg.agent.to_string(), cfg.model.clone());
    }

    let mut local_inference = LocalInferenceConfigFile {
        advanced: Some(cfg.local_inference.advanced),
        preset: Some(cfg.local_inference.preset),
        base_url: Some(cfg.local_inference.base_url.clone()),
        model: Some(cfg.local_inference.model.clone()),
        api_key: None,
    };
    if cfg.local_inference.base_url.trim().is_empty() {
        local_inference.base_url = None;
    }
    if cfg.local_inference.model.trim().is_empty() {
        local_inference.model = None;
    }

    let bot = BotSettingsFile {
        mode: Some(cfg.bot_settings.mode),
        app_id: (!cfg.bot_settings.app_id.trim().is_empty())
            .then(|| cfg.bot_settings.app_id.clone()),
        installation_id: (!cfg.bot_settings.installation_id.trim().is_empty())
            .then(|| cfg.bot_settings.installation_id.clone()),
    };

    let file_cfg = DevConfig {
        project_name: Some(cfg.project_name.clone()),
        local_inference,
        security_scan: ScanTargetsFile {
            edge: Some(cfg.scan_targets.edge.clone()),
            network_kem: Some(cfg.scan_targets.network_kem.clone()),
            network_crypto: Some(cfg.scan_targets.network_crypto.clone()),
            network: Some(cfg.scan_targets.network.clone()),
            service: Some(cfg.scan_targets.service.clone()),
            gateway: Some(cfg.scan_targets.gateway.clone()),
            gateway_users: Some(cfg.scan_targets.gateway_users.clone()),
            gateway_kms: Some(cfg.scan_targets.gateway_kms.clone()),
            cli_build: Some(cfg.scan_targets.cli_build.clone()),
            compute: Some(cfg.scan_targets.compute.clone()),
        },
        skills: SkillPathsFile {
            user_personas: Some(cfg.skill_paths.user_personas.clone()),
            issue_tracking: Some(cfg.skill_paths.issue_tracking.clone()),
        },
        bootstrap_agent_files: Some(cfg.bootstrap_agent_files),
        workflow_preset: if cfg.workflow_preset == "default" {
            None
        } else {
            Some(cfg.workflow_preset.clone())
        },
        bot,
        bootstrap_snapshot: Some(cfg.bootstrap_snapshot),
        agent_models,
    };

    let toml = toml::to_string_pretty(&file_cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, toml).map_err(|e| e.to_string())
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
        assert_eq!("cline".parse::<Agent>().unwrap(), Agent::Cline);
        assert_eq!("Codex".parse::<Agent>().unwrap(), Agent::Codex);
        assert_eq!("COPILOT".parse::<Agent>().unwrap(), Agent::Copilot);
        assert_eq!("Gemini".parse::<Agent>().unwrap(), Agent::Gemini);
        assert_eq!("grok".parse::<Agent>().unwrap(), Agent::Grok);
        assert_eq!("Junie".parse::<Agent>().unwrap(), Agent::Junie);
        assert_eq!("xai".parse::<Agent>().unwrap(), Agent::Xai);
    }

    #[test]
    fn agent_from_str_invalid() {
        assert!("gpt4".parse::<Agent>().is_err());
        assert!("".parse::<Agent>().is_err());
    }

    #[test]
    fn agent_display_roundtrip() {
        for agent in [
            Agent::Claude,
            Agent::Cline,
            Agent::Codex,
            Agent::Copilot,
            Agent::Gemini,
            Agent::Grok,
            Agent::Junie,
            Agent::Xai,
        ] {
            let s = agent.to_string();
            assert_eq!(s.parse::<Agent>().unwrap(), agent);
        }
    }

    #[test]
    fn agent_binary_names() {
        assert_eq!(Agent::Claude.binary(), "claude");
        assert_eq!(Agent::Cline.binary(), "cline");
        assert_eq!(Agent::Codex.binary(), "codex");
        assert_eq!(Agent::Copilot.binary(), "copilot");
        assert_eq!(Agent::Gemini.binary(), "gemini");
        assert_eq!(Agent::Grok.binary(), "grok");
        assert_eq!(Agent::Junie.binary(), "junie");
        assert_eq!(Agent::Xai.binary(), "copilot");
    }

    #[test]
    fn agent_co_author_contains_name() {
        assert!(Agent::Claude.co_author().contains("Claude"));
        assert!(Agent::Cline.co_author().contains("Cline"));
        assert!(Agent::Codex.co_author().contains("Codex"));
        assert!(Agent::Copilot.co_author().contains("Copilot"));
        assert!(Agent::Gemini.co_author().contains("Gemini"));
        assert!(Agent::Grok.co_author().contains("Grok"));
        assert!(Agent::Junie.co_author().contains("Junie"));
        assert!(Agent::Xai.co_author().contains("xAI"));
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
            model: String::new(),
            auto_mode: true,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bootstrap_snapshot: true,
            workflow_preset: "default".to_string(),
            bot_settings: BotSettings::default(),
            bot_credentials: None,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn skill_paths_default_points_to_app_data() {
        let p = SkillPaths::default();
        assert!(p.user_personas.contains("freq-ai"));
        assert!(p.user_personas.ends_with("skills/user-personas/SKILL.md"));
        assert!(p.issue_tracking.ends_with("skills/issue-tracking/SKILL.md"));
    }

    #[test]
    fn skill_paths_file_merges_defaults() {
        let merged = SkillPathsFile {
            user_personas: Some("/custom/skills/freq-cloud-user-personas/SKILL.md".into()),
            issue_tracking: None,
        }
        .into_skill_paths();
        assert_eq!(
            merged.user_personas,
            "/custom/skills/freq-cloud-user-personas/SKILL.md"
        );
        // Falls back to default for the field that wasn't overridden.
        assert!(
            merged
                .issue_tracking
                .ends_with("skills/issue-tracking/SKILL.md")
        );
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
            model: String::new(),
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bootstrap_snapshot: true,
            workflow_preset: "default".to_string(),
            bot_settings: BotSettings::default(),
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
            model: String::new(),
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bootstrap_snapshot: true,
            workflow_preset: "default".to_string(),
            bot_settings: BotSettings::default(),
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
            model: String::new(),
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bootstrap_snapshot: true,
            workflow_preset: "default".to_string(),
            bot_settings: BotSettings::default(),
            bot_credentials: Some(BotCredentials::GitHubApp {
                app_id: "12345".into(),
                installation_id: "67890".into(),
                private_key_pem:
                    "-----BEGIN RSA PRIVATE KEY-----\nsecret\n-----END RSA PRIVATE KEY-----".into(),
            }),
        };
        let dbg = format!("{cfg:?}");
        assert!(
            !dbg.contains("12345")
                && !dbg.contains("67890")
                && !dbg.contains("BEGIN RSA PRIVATE KEY"),
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
            private_key_pem:
                "-----BEGIN RSA PRIVATE KEY-----\nsuper-secret\n-----END RSA PRIVATE KEY-----"
                    .into(),
        };
        let dbg = format!("{app:?}");
        assert!(!dbg.contains("appid42"));
        assert!(!dbg.contains("instid99"));
        assert!(!dbg.contains("BEGIN RSA PRIVATE KEY"));
        assert!(dbg.contains("redacted"));
    }

    #[test]
    fn config_serde_skips_bot_credentials_github_app() {
        let cfg = Config {
            agent: Agent::Claude,
            model: String::new(),
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig::default(),
            root: "/tmp/test".into(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bootstrap_snapshot: true,
            workflow_preset: "default".to_string(),
            bot_settings: BotSettings::default(),
            bot_credentials: Some(BotCredentials::GitHubApp {
                app_id: "12345".into(),
                installation_id: "67890".into(),
                private_key_pem:
                    "-----BEGIN RSA PRIVATE KEY-----\nsecret\n-----END RSA PRIVATE KEY-----".into(),
            }),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            !json.contains("BEGIN RSA PRIVATE KEY") && !json.contains("12345"),
            "GitHub App credentials must not appear in serialized Config"
        );
        let back: Config = serde_json::from_str(&json).unwrap();
        assert!(back.bot_credentials.is_none());
    }

    #[test]
    fn bot_settings_require_complete_selected_mode() {
        let mut settings = BotSettings {
            mode: BotAuthMode::Token,
            token: "github_pat_123".into(),
            ..BotSettings::default()
        };
        assert!(matches!(
            settings.to_credentials(),
            Some(BotCredentials::Token(_))
        ));

        settings.token.clear();
        assert!(settings.to_credentials().is_none());

        settings.mode = BotAuthMode::GitHubApp;
        settings.app_id = "123".into();
        settings.installation_id = "456".into();
        settings.private_key_pem =
            "-----BEGIN RSA PRIVATE KEY-----\nsecret\n-----END RSA PRIVATE KEY-----".into();
        assert!(matches!(
            settings.to_credentials(),
            Some(BotCredentials::GitHubApp { .. })
        ));
    }

    #[test]
    fn save_dev_config_omits_plaintext_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            agent: Agent::Claude,
            model: String::new(),
            auto_mode: false,
            dry_run: false,
            local_inference: LocalInferenceConfig {
                api_key: "super-secret-local-key".into(),
                ..LocalInferenceConfig::default()
            },
            root: dir.path().to_string_lossy().into_owned(),
            project_name: "my-project".into(),
            scan_targets: ScanTargets::default(),
            skill_paths: SkillPaths::default(),
            bootstrap_agent_files: true,
            bootstrap_snapshot: true,
            workflow_preset: "default".to_string(),
            bot_settings: BotSettings {
                mode: BotAuthMode::GitHubApp,
                app_id: "12345".into(),
                installation_id: "67890".into(),
                private_key_pem: "-----BEGIN RSA PRIVATE KEY-----\nsuper-secret-pem\n-----END RSA PRIVATE KEY-----".into(),
                ..BotSettings::default()
            },
            bot_credentials: None,
        };

        save_dev_config(&cfg.root, &cfg).unwrap();
        let saved = std::fs::read_to_string(dir.path().join("dev.toml")).unwrap();
        assert!(!saved.contains("super-secret-local-key"));
        assert!(!saved.contains("super-secret-pem"));
        assert!(saved.contains("[bot]"));
        assert!(saved.contains("mode = \"github_app\""));
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
