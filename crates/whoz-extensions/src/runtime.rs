use crate::content_scripts::ContentScriptRule;
use crate::manifest::ExtensionManifest;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ExtensionInstance {
    pub id: String,
    pub manifest: ExtensionManifest,
    pub path: PathBuf,
    pub is_enabled: bool,
    pub is_service_worker_active: bool,
}

pub struct ExtensionManager {
    extensions: HashMap<String, ExtensionInstance>,
    content_rules: Vec<ContentScriptRule>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            content_rules: Vec::new(),
        }
    }

    pub fn register(&mut self, id: String, manifest: ExtensionManifest, path: PathBuf) {
        // Collect content scripts
        if let Some(scripts) = &manifest.content_scripts {
            for entry in scripts {
                let js_files = entry
                    .js
                    .as_ref()
                    .map(|files| files.iter().map(|f| path.join(f)).collect())
                    .unwrap_or_default();

                let css_files = entry
                    .css
                    .as_ref()
                    .map(|files| files.iter().map(|f| path.join(f)).collect())
                    .unwrap_or_default();

                self.content_rules.push(ContentScriptRule {
                    extension_id: id.clone(),
                    match_patterns: entry.matches.clone(),
                    js_files,
                    css_files,
                });
            }
        }

        let instance = ExtensionInstance {
            id: id.clone(),
            manifest,
            path,
            is_enabled: true,
            is_service_worker_active: false, // Suspended by default until an event arrives
        };

        self.extensions.insert(id, instance);
    }

    /// Finds matching content scripts to inject into a given page URL
    pub fn get_scripts_for_url(&self, url: &str) -> Vec<&ContentScriptRule> {
        self.content_rules
            .iter()
            .filter(|rule| rule.matches_url(url))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.extensions.len()
    }
}
