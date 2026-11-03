use crate::tab::{Tab, TabId, TabState};
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_allocated_kb: usize,
    pub active_tabs_count: usize,
    pub suspended_tabs_count: usize,
    pub discarded_tabs_count: usize,
    pub saved_memory_kb: usize,
}

#[derive(Debug, Clone)]
pub struct TabSuspensionPolicy {
    pub idle_timeout: Duration,
    pub ignore_audio_playing: bool,
    pub ignore_pinned_tabs: bool,
    pub memory_limit_mb: usize,
}

impl Default for TabSuspensionPolicy {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(60),
            ignore_audio_playing: true,
            ignore_pinned_tabs: true,
            memory_limit_mb: 800,
        }
    }
}

pub struct MemoryGovernor {
    policy: TabSuspensionPolicy,
}

impl MemoryGovernor {
    pub fn new(policy: TabSuspensionPolicy) -> Self {
        Self { policy }
    }

    /// Evaluates tab list and applies suspension to idle tabs
    pub fn evaluate_and_reclaim(&self, tabs: &mut HashMap<TabId, Tab>, current_active_id: Option<TabId>) -> MemoryStats {
        let mut total_kb = 0;
        let mut active_count = 0;
        let mut suspended_count = 0;
        let mut discarded_count = 0;
        let mut saved_kb = 0;

        for (id, tab) in tabs.iter_mut() {
            let is_current = current_active_id == Some(*id);

            if is_current {
                tab.state = TabState::Active;
                active_count += 1;
                total_kb += tab.estimated_memory_kb;
                continue;
            }

            // Exemption check
            if (self.policy.ignore_audio_playing && tab.is_audio_playing)
                || (self.policy.ignore_pinned_tabs && tab.is_pinned)
            {
                tab.state = TabState::Background;
                active_count += 1;
                total_kb += tab.estimated_memory_kb;
                continue;
            }

            // Check if eligible for suspension
            if tab.last_active.elapsed() >= self.policy.idle_timeout {
                if tab.state == TabState::Active || tab.state == TabState::Background {
                    info!(
                        tab_id = id,
                        url = %tab.url,
                        "Suspending idle tab to reclaim memory"
                    );
                    tab.state = TabState::Suspended;
                    // Retain serialized snapshot, drop V8 Heap
                    saved_kb += tab.estimated_memory_kb.saturating_sub(2_000); // Only keep ~2MB stub
                    tab.estimated_memory_kb = 2_000;
                }
            }

            match tab.state {
                TabState::Active | TabState::Background => active_count += 1,
                TabState::Suspended => suspended_count += 1,
                TabState::Discarded => discarded_count += 1,
            }

            total_kb += tab.estimated_memory_kb;
        }

        MemoryStats {
            total_allocated_kb: total_kb,
            active_tabs_count: active_count,
            suspended_tabs_count: suspended_count,
            discarded_tabs_count: discarded_count,
            saved_memory_kb: saved_kb,
        }
    }
}
