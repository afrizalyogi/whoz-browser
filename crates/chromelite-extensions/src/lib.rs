//! Chromelite Extensions: Lean Manifest V3/V2 Chrome Extension runtime in Rust.

pub mod content_scripts;
pub mod loader;
pub mod manifest;
pub mod runtime;

pub use content_scripts::ContentScriptRule;
pub use loader::ExtensionLoader;
pub use manifest::{ExtensionManifest, ManifestVersion};
pub use runtime::{ExtensionInstance, ExtensionManager};
