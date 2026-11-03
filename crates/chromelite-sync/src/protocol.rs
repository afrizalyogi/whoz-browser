use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncDataType {
    Bookmarks,
    Preferences,
    Passwords,
    Autofill,
    Extensions,
    Themes,
    HistoryDeleteDirectives,
    Sessions,
    Nigori,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkItem {
    pub id: String,
    pub parent_id: String,
    pub title: String,
    pub url: String,
    pub is_folder: bool,
    pub position_in_parent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntity {
    pub id: String,
    pub data_type: SyncDataType,
    pub version: i64,
    pub mtime_ms: u64,
    pub name: String,
    pub is_deleted: bool,
    pub payload_json: String,
}
