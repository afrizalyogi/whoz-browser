use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestVersion {
    V2 = 2,
    V3 = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundServiceWorker {
    pub service_worker: String,
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentScriptEntry {
    pub matches: Vec<String>,
    pub js: Option<Vec<String>>,
    pub css: Option<Vec<String>>,
    pub run_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub host_permissions: Option<Vec<String>>,
    pub background: Option<serde_json::Value>,
    pub content_scripts: Option<Vec<ContentScriptEntry>>,
}

impl ExtensionManifest {
    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn is_mv3(&self) -> bool {
        self.manifest_version == 3
    }
}
