use rust_embed::RustEmbed;
use std::path::PathBuf;

pub const AGENTS_MD: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/AGENTS.md"));
pub const LABELS_YML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/labels.yml"));

#[derive(RustEmbed)]
#[folder = "assets/skills/"]
pub struct SkillAssets;

#[derive(RustEmbed)]
#[folder = "assets/workflows/"]
pub struct WorkflowAssets;

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
/// Existing files are refreshed so the bundled guidance stays in sync with
/// the current binary.
/// Returns the app-data root (e.g. `~/.local/share/freq-ai`).
pub fn materialize_assets() -> PathBuf {
    let dir = assets_dir();

    // 1. AGENTS.md
    let agents_md = dir.join("AGENTS.md");
    let _ = std::fs::write(&agents_md, AGENTS_MD.as_bytes());

    // 2. Skills
    for file in SkillAssets::iter() {
        let path = dir.join("skills").join(file.as_ref());
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Some(embedded) = SkillAssets::get(file.as_ref()) {
            let _ = std::fs::write(&path, embedded.data);
        }
    }

    // 3. Workflows
    for file in WorkflowAssets::iter() {
        let path = dir.join("workflows").join(file.as_ref());
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Some(embedded) = WorkflowAssets::get(file.as_ref()) {
            let _ = std::fs::write(&path, embedded.data);
        }
    }

    dir
}
