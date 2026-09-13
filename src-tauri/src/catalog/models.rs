use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SourceRoot {
    pub id: i64,
    pub path: String,
    pub kind: String,
    pub scan_policy: String,
    pub poll_secs: Option<i64>,
    pub last_scan_at: Option<i64>,
    pub status: String,
    pub smb_host: Option<String>,
    pub smb_share: Option<String>,
    pub smb_username: Option<String>,
    pub smb_mounted: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Asset {
    pub id: i64,
    pub root_id: i64,
    pub rel_path: String,
    pub file_name: String,
    pub ext: String,
    pub kind: String,
    pub size: i64,
    pub mtime_ns: i64,
    pub content_hash: Option<String>,
    pub thumb_key: Option<String>,
    pub indexed_mtime_ns: Option<i64>,
    pub sync_state: String,
    pub deleted_at: Option<i64>,
    pub has_duplicate: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AssetScanState {
    pub rel_path: String,
    pub mtime_ns: i64,
    pub size: i64,
    pub indexed_mtime_ns: Option<i64>,
    pub thumb_key: Option<String>,
    pub content_hash: Option<String>,
    pub kind: String,
    pub raw_tag_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AssetMeta {
    pub asset_id: i64,
    pub capture_at: Option<i64>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub rating: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub keywords_json: Option<String>,
    pub rotation: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetCard {
    pub id: i64,
    pub file_name: String,
    pub ext: String,
    pub kind: String,
    pub capture_at: Option<i64>,
    pub rating: Option<i64>,
    pub sync_state: String,
    pub thumb_path: Option<String>,
    pub abs_path: String,
    pub has_duplicate: bool,
    pub rotation: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DuplicateAsset {
    pub id: i64,
    pub file_name: String,
    pub root_path: String,
    pub rel_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDetail {
    pub asset: Asset,
    pub meta: Option<AssetMeta>,
    pub abs_path: String,
    pub display_path: String,
    pub tag_ids: Vec<i64>,
    pub album_ids: Vec<i64>,
    pub raw_tags: Vec<RawTag>,
    pub links: Vec<LinkedAsset>,
    pub duplicates: Vec<DuplicateAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RawTag {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub id: i64,
    pub status: String,
    pub manifest_json: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Album {
    pub id: i64,
    pub name: String,
    pub sort_mode: String,
    pub emoji: Option<String>,
    pub asset_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCollection {
    pub id: i64,
    pub name: String,
    pub filter: crate::query::AssetFilter,
    pub asset_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SmartCollectionRow {
    pub id: i64,
    pub name: String,
    pub filter_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LinkedAsset {
    pub kind: String,
    pub id: i64,
    pub file_name: String,
    pub asset_kind: String,
    pub root_path: String,
    pub rel_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub flat: bool,
    pub rename_template: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetMetaPatch {
    pub rating: Option<i64>,
    pub rotation: Option<i64>,
}
