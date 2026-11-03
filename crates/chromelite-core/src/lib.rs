//! Chromelite Core: High-efficiency browser governor and memory controller.

pub mod config;
pub mod memory;
pub mod process;
pub mod tab;

pub use config::ChromeliteConfig;
pub use memory::{MemoryGovernor, MemoryStats, TabSuspensionPolicy};
pub use process::{ProcessPool, ProcessSupervisor, RendererProcessId};
pub use tab::{Tab, TabId, TabState};
