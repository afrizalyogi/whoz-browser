//! Chromelite UI: Ultra-lightweight native Rust UI shell for Chromium engine.

pub mod native_pages;
pub mod theme;
pub mod window;

pub use native_pages::{NativePageKind, NativePagesManager};
pub use theme::ThemeConfig;
pub use window::BrowserWindowShell;
