use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ContentScriptRule {
    pub extension_id: String,
    pub match_patterns: Vec<String>,
    pub js_files: Vec<PathBuf>,
    pub css_files: Vec<PathBuf>,
}

impl ContentScriptRule {
    /// Simple match pattern evaluator (supports `*://*/*`, `<all_urls>`, exact domain matches)
    pub fn matches_url(&self, url: &str) -> bool {
        for pattern in &self.match_patterns {
            if pattern == "<all_urls>" {
                return true;
            }
            if pattern == "*://*/*" && (url.starts_with("http://") || url.starts_with("https://")) {
                return true;
            }
            // Domain substring match
            let domain_clean = pattern.replace("*://", "").replace("/*", "");
            if url.contains(&domain_clean) {
                return true;
            }
        }
        false
    }
}
