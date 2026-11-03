use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhozConfig {
    /// Maximum allowed renderer processes in pool before sharing isolates
    pub max_renderer_processes: usize,
    /// Idle duration in seconds before tab is automatically suspended
    pub tab_suspend_timeout_secs: u64,
    /// Memory pressure threshold in MB triggering aggressive memory reclamation
    pub memory_pressure_threshold_mb: usize,
    /// Whether to strip Google telemetry, UMA, and crash reports
    pub disable_telemetry: bool,
    /// Whether to bypass WebUI renderers for internal pages and use native UI
    pub use_native_internal_pages: bool,
    /// Whether to enable Google Sync
    pub enable_google_sync: bool,
    /// Path to user data directory
    pub user_data_dir: PathBuf,
    /// Path to base Chrome installation (read-only reference)
    pub chrome_base_dir: Option<PathBuf>,
}

impl Default for WhozConfig {
    fn default() -> Self {
        Self {
            max_renderer_processes: 4,
            tab_suspend_timeout_secs: 60,
            memory_pressure_threshold_mb: 800,
            disable_telemetry: true,
            use_native_internal_pages: true,
            enable_google_sync: true,
            user_data_dir: PathBuf::from("./data/user_data"),
            chrome_base_dir: Some(PathBuf::from(r"C:\Program Files\Google\Chrome\Application")),
        }
    }
}
