use crate::manifest::ExtensionManifest;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

pub struct ExtensionLoader;

impl ExtensionLoader {
    /// Loads unpacked extension from folder
    pub fn load_from_dir(dir: &Path) -> anyhow::Result<(String, ExtensionManifest, PathBuf)> {
        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            anyhow::bail!("manifest.json not found in {}", dir.display());
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest = ExtensionManifest::parse(&content)?;
        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_ext")
            .to_string();

        info!(
            extension_id = %id,
            name = %manifest.name,
            version = %manifest.version,
            mv3 = manifest.is_mv3(),
            "Loaded Chrome extension"
        );

        Ok((id, manifest, dir.to_path_buf()))
    }
}
