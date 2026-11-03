use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativePageKind {
    Settings,
    Bookmarks,
    History,
    Extensions,
    Downloads,
    SystemStatus,
}

pub struct NativePagesManager;

impl NativePagesManager {
    /// Intercepts `chrome://` URLs and serves them natively in Rust without WebUI renderers
    pub fn match_native_url(url: &str) -> Option<NativePageKind> {
        match url {
            "chrome://settings" | "chromelite://settings" => Some(NativePageKind::Settings),
            "chrome://bookmarks" | "chromelite://bookmarks" => Some(NativePageKind::Bookmarks),
            "chrome://history" | "chromelite://history" => Some(NativePageKind::History),
            "chrome://extensions" | "chromelite://extensions" => Some(NativePageKind::Extensions),
            "chrome://downloads" | "chromelite://downloads" => Some(NativePageKind::Downloads),
            "chrome://system" | "chromelite://system" => Some(NativePageKind::SystemStatus),
            _ => None,
        }
    }
}
