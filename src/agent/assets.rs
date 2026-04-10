use rust_embed::RustEmbed;

pub const AGENTS_MD: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/AGENTS.md"));

#[derive(RustEmbed)]
#[folder = "assets/skills/"]
pub struct SkillAssets;
