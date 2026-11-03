/// Generates the stripped, optimized command-line flags for Chromium engine.
pub struct ChromiumFlagsBuilder {
    flags: Vec<String>,
}

impl ChromiumFlagsBuilder {
    pub fn new() -> Self {
        let mut builder = Self { flags: Vec::new() };
        builder.apply_essential_optimizations();
        builder
    }

    /// Strips known telemetry, background hogs, and bloat discovered in REA analysis
    fn apply_essential_optimizations(&mut self) {
        self.flags.extend([
            // 1. Process & Memory Governors
            "--renderer-process-limit=4".to_string(),
            "--js-flags=--max-old-space-size=256".to_string(),
            "--enable-aggressive-domstorage-flushing".to_string(),
            "--disable-smooth-scrolling".to_string(),

            // 2. Disable Background & Telemetry Services
            "--disable-breakpad".to_string(),
            "--disable-crash-reporter".to_string(),
            "--disable-background-networking".to_string(),
            "--disable-component-update".to_string(),
            "--disable-domain-reliability".to_string(),
            "--disable-sync".to_string(), // Handled by Rust Whoz Sync with 95% less RAM!

            // 3. Disable Heavy ML / AI Guides found in REA (optimization_guide_internal.dll)
            "--disable-features=OptimizationHints,OptimizationGuideModelDownloading,MediaRouter,Translate,AutofillServerCommunication,InterestFeedContentSuggestions".to_string(),

            // 4. Disable heavy WebUI background renders
            "--disable-default-apps".to_string(),
            "--no-default-browser-check".to_string(),
        ]);
    }

    pub fn with_user_data_dir(mut self, path: &str) -> Self {
        self.flags.push(format!("--user-data-dir={}", path));
        self
    }

    pub fn build(self) -> Vec<String> {
        self.flags
    }
}
