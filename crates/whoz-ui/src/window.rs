use crate::theme::ThemeConfig;
use whoz_core::TabId;
use tracing::info;

pub struct BrowserWindowShell {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub current_url: String,
    pub active_tab_id: Option<TabId>,
    pub theme: ThemeConfig,
}

impl BrowserWindowShell {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        Self {
            title: title.to_string(),
            width,
            height,
            current_url: "https://www.google.com".to_string(),
            active_tab_id: None,
            theme: ThemeConfig::default(),
        }
    }

    pub fn set_active_tab(&mut self, tab_id: TabId, url: String) {
        self.active_tab_id = Some(tab_id);
        self.current_url = url;
        info!(tab_id, url = %self.current_url, "UI active tab updated");
    }
}
