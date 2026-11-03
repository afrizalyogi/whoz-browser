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
            "chrome://settings" | "whoz://settings" => Some(NativePageKind::Settings),
            "chrome://bookmarks" | "whoz://bookmarks" => Some(NativePageKind::Bookmarks),
            "chrome://history" | "whoz://history" => Some(NativePageKind::History),
            "chrome://extensions" | "whoz://extensions" => Some(NativePageKind::Extensions),
            "chrome://downloads" | "whoz://downloads" => Some(NativePageKind::Downloads),
            "chrome://system" | "whoz://system" => Some(NativePageKind::SystemStatus),
            _ => None,
        }
    }
}
