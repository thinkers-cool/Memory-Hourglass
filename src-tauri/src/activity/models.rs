use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: i64,
    pub seq: i64,
    pub occurred_at: i64,
    pub event_type: String,
    pub actor: String,
    pub correlation_id: Option<String>,
    pub subject_type: Option<String>,
    pub subject_id: Option<i64>,
    pub subject_key: Option<String>,
    pub summary: Option<String>,
    pub payload_json: String,
    pub revert_json: Option<String>,
    pub undone_at: Option<i64>,
    pub reversible: bool,
}

#[derive(Debug, Clone)]
pub struct ActivityInput {
    pub event_type: String,
    pub actor: String,
    pub correlation_id: Option<String>,
    pub subject_type: Option<String>,
    pub subject_id: Option<i64>,
    pub subject_key: Option<String>,
    pub summary: Option<String>,
    pub payload_json: String,
    pub revert_json: Option<String>,
}

pub mod event_type {
    pub const ASSET_METADATA_CHANGED: &str = "asset.metadata_changed";
    pub const ASSET_TAGS_ADDED: &str = "asset.tags_added";
    pub const ASSET_TAGS_REMOVED: &str = "asset.tags_removed";
    pub const ASSET_SOFT_DELETED: &str = "asset.soft_deleted";
    pub const ASSET_RESTORED: &str = "asset.restored";
    pub const ASSET_PURGED: &str = "asset.purged";
    pub const ASSET_DISCOVERED: &str = "asset.discovered";
    pub const ASSET_MODIFIED: &str = "asset.modified";
    pub const ASSET_MISSING: &str = "asset.missing";
    pub const SCAN_COMPLETED: &str = "scan.completed";
    pub const EXPORT_COMPLETED: &str = "export.completed";
    pub const CATALOG_REBUILT: &str = "catalog.rebuilt";
    pub const ALBUM_ITEMS_CHANGED: &str = "album.items_changed";
}

pub mod actor {
    pub const USER: &str = "user";
    pub const SCAN: &str = "scan";
    pub const WATCHER: &str = "watcher";
    pub const EXPORT: &str = "export";
    pub const SYSTEM: &str = "system";
}
