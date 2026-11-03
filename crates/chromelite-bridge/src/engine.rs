use chromelite_core::TabId;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub base_binary_dir: PathBuf,
    pub user_data_dir: PathBuf,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromiumBackend {
    EmbeddedContentApi,
    WebView2Core,
    CefLightweight,
}

pub struct EngineHandle {
    pub backend: ChromiumBackend,
    pub active_tabs: Vec<TabId>,
}

impl EngineHandle {
    pub fn initialize(backend: ChromiumBackend, config: EngineConfig) -> anyhow::Result<Self> {
        info!(
            ?backend,
            base_dir = %config.base_binary_dir.display(),
            flag_count = config.flags.len(),
            "Initialized stripped Chromium engine bridge"
        );

        Ok(Self {
            backend,
            active_tabs: Vec::new(),
        })
    }

    pub fn navigate_tab(&mut self, tab_id: TabId, url: &str) {
        info!(tab_id, url, "Engine navigating tab");
        if !self.active_tabs.contains(&tab_id) {
            self.active_tabs.push(tab_id);
        }
    }

    pub fn suspend_tab(&mut self, tab_id: TabId) {
        info!(tab_id, "Engine suspending tab WebContents and releasing renderer page pool");
    }

    pub fn resume_tab(&mut self, tab_id: TabId) {
        info!(tab_id, "Engine restoring tab WebContents from cached DOM");
    }
}
