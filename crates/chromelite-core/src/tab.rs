use serde::{Deserialize, Serialize};
use std::time::Instant;

pub type TabId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabState {
    Active,
    Background,
    Suspended,
    Discarded,
}

#[derive(Debug)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub favicon_url: Option<String>,
    pub state: TabState,
    pub last_active: Instant,
    pub estimated_memory_kb: usize,
    pub is_pinned: bool,
    pub is_audio_playing: bool,
}

impl Tab {
    pub fn new(id: TabId, url: String) -> Self {
        Self {
            id,
            title: "New Tab".to_string(),
            url,
            favicon_url: None,
            state: TabState::Active,
            last_active: Instant::now(),
            estimated_memory_kb: 45_000, // ~45 MB initial estimate
            is_pinned: false,
            is_audio_playing: false,
        }
    }

    pub fn touch(&mut self) {
        self.last_active = Instant::now();
        self.state = TabState::Active;
    }

    pub fn idle_duration_secs(&self) -> u64 {
        self.last_active.elapsed().as_secs()
    }
}
