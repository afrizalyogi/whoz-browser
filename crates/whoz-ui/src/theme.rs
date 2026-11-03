use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub dark_mode: bool,
    pub accent_color: String,
    pub background_color: String,
    pub tab_bar_height: u32,
    pub omnibox_height: u32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            dark_mode: true,
            accent_color: "#1a73e8".to_string(),
            background_color: "#202124".to_string(),
            tab_bar_height: 38,
            omnibox_height: 34,
        }
    }
}
