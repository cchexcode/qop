use {anyhow::{Context, Result}, std::path::Path};

// This module is deprecated. Config builders now live in subsystem modules.
// Retain only write_config for shared file writing.

pub fn write_config(path: &Path, toml_str: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| anyhow::anyhow!("invalid path"))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    std::fs::write(path, toml_str)
        .with_context(|| format!("Failed to write config file to: {}", path.display()))?;
    Ok(())
}
