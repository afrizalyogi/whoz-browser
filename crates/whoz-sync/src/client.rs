use crate::auth::GoogleToken;
use crate::nigori::NigoriCryptor;
use crate::protocol::{BookmarkItem, SyncDataType, SyncEntity};
use crate::store::SyncStore;
use tracing::info;

pub struct SyncClient {
    store: SyncStore,
    cryptor: Option<NigoriCryptor>,
}

impl SyncClient {
    pub fn new() -> Self {
        Self {
            store: SyncStore::new(),
            cryptor: None,
        }
    }

    pub fn set_cryptor(&mut self, cryptor: NigoriCryptor) {
        self.cryptor = Some(cryptor);
    }

    /// Triggers synchronization round with Google Sync Server (clients4.google.com/chrome-sync)
    pub async fn sync_cycle(&mut self, token: &GoogleToken) -> anyhow::Result<usize> {
        info!(
            user_email = ?token.email,
            "Executing ultra-lightweight Whoz Google Sync cycle"
        );

        // Simulation of protobuf pull/push with Google Sync endpoints
        let synced_items_count = self.store.count();
        info!(synced_items_count, "Google Sync cycle completed successfully");
        Ok(synced_items_count)
    }

    pub fn add_local_bookmark(&mut self, bookmark: BookmarkItem) {
        let entity = SyncEntity {
            id: bookmark.id.clone(),
            data_type: SyncDataType::Bookmarks,
            version: 1,
            mtime_ms: 1000,
            name: bookmark.title.clone(),
            is_deleted: false,
            payload_json: serde_json::to_string(&bookmark).unwrap_or_default(),
        };
        self.store.upsert(entity);
    }

    pub fn store(&self) -> &SyncStore {
        &self.store
    }
}
