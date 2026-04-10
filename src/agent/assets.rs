use rust_embed::RustEmbed;
use std::path::PathBuf;

pub const AGENTS_MD: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/AGENTS.md"));

#[derive(RustEmbed)]
#[folder = "assets/skills/"]
pub struct SkillAssets;

/// Return the stable app-data directory for materialized assets
/// (`~/.local/share/freq-ai`). Created on first call if missing.
pub fn assets_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("freq-ai");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Materialize embedded AGENTS.md and skills into the app-data directory.
/// Existing files are left untouched so user edits are preserved.
/// Returns the app-data root (e.g. `~/.local/share/freq-ai`).
pub fn materialize_assets() -> PathBuf {
    let dir = assets_dir();

    // 1. AGENTS.md
    let agents_md = dir.join("AGENTS.md");
    if !agents_md.exists() {
        let _ = std::fs::write(&agents_md, AGENTS_MD.as_bytes());
    }

    // 2. Skills
    for file in SkillAssets::iter() {
        let path = dir.join("skills").join(file.as_ref());
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Some(embedded) = SkillAssets::get(file.as_ref()) {
                let _ = std::fs::write(&path, embedded.data);
            }
        }
    }

    dir
}
