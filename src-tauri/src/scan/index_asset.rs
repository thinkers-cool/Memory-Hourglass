use crate::catalog::models::{AssetMeta, RawTag};
use crate::link::sha256_bytes;
use crate::metadata::{MetadataContext, MetadataService};
use crate::scan::index_integrity::is_index_complete;
use crate::scan::IoProfile;
use crate::thumb::ThumbService;
use exiftool_rs::ExifTool;
use rayon::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Instant;

thread_local! {
    static THREAD_EXIF: RefCell<Option<ExifTool>> = const { RefCell::new(None) };
}

#[derive(Debug)]
pub struct IndexOutput {
    pub meta: AssetMeta,
    pub raw_tags: Vec<RawTag>,
    pub thumb_key: Option<String>,
    pub content_hash: Option<String>,
    pub skipped: bool,
}

pub struct BatchIndexItem {
    pub asset_id: i64,
    pub path: PathBuf,
    pub metadata_ctx: MetadataContext,
    pub mtime_ns: i64,
    pub kind: String,
    pub prior_thumb_key: Option<String>,
    pub prior_indexed_mtime_ns: Option<i64>,
    pub prior_content_hash: Option<String>,
}

pub fn index_parallelism(profile: IoProfile) -> usize {
    match profile {
        IoProfile::Local => std::thread::available_parallelism()
            .map(|count| (count.get() / 2).clamp(2, 4))
            .unwrap_or(2),
        IoProfile::Network => 2,
    }
}

fn empty_meta(asset_id: i64) -> AssetMeta {
    AssetMeta {
        asset_id,
        capture_at: None,
        camera: None,
        lens: None,
        rating: None,
        latitude: None,
        longitude: None,
        keywords_json: None,
        rotation: None,
    }
}

fn with_thread_exif_tool<F, R>(f: F) -> R
where
    F: FnOnce(&ExifTool) -> R,
{
    THREAD_EXIF.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(ExifTool::new());
        }
        f(slot.as_ref().expect("exif tool"))
    })
}

fn existing_thumb_key(
    asset_id: i64,
    thumb_dir: &Path,
    prior_thumb_key: Option<&str>,
) -> Option<String> {
    let thumb_svc = ThumbService::new(thumb_dir.to_path_buf());
    if thumb_svc.thumb_path(asset_id).exists() {
        return thumb_svc
            .thumb_path(asset_id)
            .file_name()
            .map(|name| name.to_string_lossy().to_string());
    }
    prior_thumb_key.map(|key| key.to_string())
}

pub fn can_skip_index(
    mtime_ns: i64,
    kind: &str,
    thumb_dir: &Path,
    asset_id: i64,
    prior_thumb_key: Option<&str>,
    prior_indexed_mtime_ns: Option<i64>,
    prior_content_hash: Option<&str>,
) -> bool {
    if prior_indexed_mtime_ns != Some(mtime_ns) {
        return false;
    }
    if prior_content_hash.is_none() {
        return false;
    }
    let thumb_key = existing_thumb_key(asset_id, thumb_dir, prior_thumb_key);
    is_index_complete(mtime_ns, prior_indexed_mtime_ns, thumb_key.as_deref(), kind)
}

pub fn index_file(
    asset_id: i64,
    path: &Path,
    thumb_dir: &Path,
    metadata_ctx: &MetadataContext,
    mtime_ns: i64,
    kind: &str,
    prior_thumb_key: Option<&str>,
    prior_indexed_mtime_ns: Option<i64>,
    prior_content_hash: Option<&str>,
) -> IndexOutput {
    if can_skip_index(
        mtime_ns,
        kind,
        thumb_dir,
        asset_id,
        prior_thumb_key,
        prior_indexed_mtime_ns,
        prior_content_hash,
    ) {
        let thumb_key = existing_thumb_key(asset_id, thumb_dir, prior_thumb_key);
        tracing::debug!(
            asset_id,
            path = %path.display(),
            "index file skipped"
        );
        return IndexOutput {
            meta: empty_meta(asset_id),
            raw_tags: Vec::new(),
            thumb_key,
            content_hash: prior_content_hash.map(str::to_string),
            skipped: true,
        };
    }

    let total_start = Instant::now();

    let read_start = Instant::now();
    let bytes = std::fs::read(path).ok();
    let read_ms = read_start.elapsed().as_millis() as u64;

    let hash_start = Instant::now();
    let content_hash = bytes.as_ref().map(|data| sha256_bytes(data));
    let hash_ms = hash_start.elapsed().as_millis() as u64;

    if let (Some(stored_hash), Some(computed_hash)) = (prior_content_hash, content_hash.as_deref()) {
        if stored_hash == computed_hash
            && prior_indexed_mtime_ns == Some(mtime_ns)
            && existing_thumb_key(asset_id, thumb_dir, prior_thumb_key).is_some()
        {
            tracing::debug!(
                asset_id,
                path = %path.display(),
                read_ms,
                hash_ms,
                total_ms = total_start.elapsed().as_millis() as u64,
                "index file skipped after hash match"
            );
            return IndexOutput {
                meta: empty_meta(asset_id),
                raw_tags: Vec::new(),
                thumb_key: existing_thumb_key(asset_id, thumb_dir, prior_thumb_key),
                content_hash: Some(stored_hash.to_string()),
                skipped: true,
            };
        }
    }

    let exif_start = Instant::now();
    let exif_data = bytes.as_ref().map(|data| {
        with_thread_exif_tool(|et| {
            MetadataService::read_index_exif_with_bytes(et, metadata_ctx, asset_id, data)
        })
    });
    let exif_ms = exif_start.elapsed().as_millis() as u64;

    let (meta, raw_tags, embedded_exif_thumb) = match exif_data {
        Some(data) => (data.meta, data.raw_tags, data.embedded_thumbnail),
        None => (empty_meta(asset_id), Vec::new(), None),
    };

    let thumb_start = Instant::now();
    let thumb_svc = ThumbService::new(thumb_dir.to_path_buf());
    let thumb_key = bytes
        .as_ref()
        .and_then(|data| {
            thumb_svc
                .ensure_thumbnail_from_bytes_with_embedded(
                    asset_id,
                    path,
                    data,
                    embedded_exif_thumb.as_deref(),
                )
                .ok()
        });
    let thumb_ms = thumb_start.elapsed().as_millis() as u64;

    tracing::debug!(
        asset_id,
        path = %path.display(),
        bytes = bytes.as_ref().map(|data| data.len()).unwrap_or(0),
        read_ms,
        hash_ms,
        exif_ms,
        thumb_ms,
        used_exif_thumb = embedded_exif_thumb.is_some(),
        total_ms = total_start.elapsed().as_millis() as u64,
        has_thumb = thumb_key.is_some(),
        "index file"
    );

    IndexOutput {
        meta,
        raw_tags,
        thumb_key,
        content_hash,
        skipped: false,
    }
}

pub fn index_batch_on_disk(
    items: &[BatchIndexItem],
    thumb_dir: &Path,
    profile: IoProfile,
) -> Vec<(i64, IndexOutput)> {
    if items.is_empty() {
        return Vec::new();
    }

    let parallelism = index_parallelism(profile);
    let batch_start = Instant::now();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parallelism)
        .build()
        .expect("index thread pool");

    let results = pool.install(|| {
        items
            .par_iter()
            .map(|item| {
                (
                    item.asset_id,
                    index_file(
                        item.asset_id,
                        &item.path,
                        thumb_dir,
                        &item.metadata_ctx,
                        item.mtime_ns,
                        &item.kind,
                        item.prior_thumb_key.as_deref(),
                        item.prior_indexed_mtime_ns,
                        item.prior_content_hash.as_deref(),
                    ),
                )
            })
            .collect()
    });

    tracing::info!(
        files = items.len(),
        parallelism,
        io_profile = ?profile,
        elapsed_ms = batch_start.elapsed().as_millis() as u64,
        "index batch on disk"
    );

    results
}

pub fn index_asset_on_disk(
    asset_id: i64,
    path: &Path,
    thumb_dir: &Path,
    metadata_ctx: &MetadataContext,
) -> IndexOutput {
    index_file(
        asset_id,
        path,
        thumb_dir,
        metadata_ctx,
        0,
        "image",
        None,
        None,
        None,
    )
}

pub fn resolved_thumb_key<'a>(
    new_thumb_key: Option<&'a str>,
    prior_thumb_key: Option<&'a str>,
) -> Option<&'a str> {
    new_thumb_key.or(prior_thumb_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolved_thumb_key_prefers_new_value() {
        assert_eq!(
            resolved_thumb_key(Some("new.webp"), Some("old.webp")),
            Some("new.webp")
        );
    }

    #[test]
    fn resolved_thumb_key_falls_back_to_prior_value() {
        assert_eq!(
            resolved_thumb_key(None, Some("old.webp")),
            Some("old.webp")
        );
        assert_eq!(resolved_thumb_key(None, None), None);
    }

    #[test]
    fn indexes_jpeg_metadata_thumbnail_and_hash() {
        let dir = tempdir().unwrap();
        let photo = dir.path().join("sample.jpg");
        std::fs::write(&photo, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();

        let thumb_dir = dir.path().join("thumbs");
        let ctx = MetadataContext::in_place(photo.clone());
        let output = index_file(42, &photo, &thumb_dir, &ctx, 0, "image", None, None, None);
        assert_eq!(output.meta.asset_id, 42);
        assert_eq!(output.thumb_key.as_deref(), Some("42.webp"));
        assert!(thumb_dir.join("42.webp").exists());
        assert!(output.content_hash.is_some());
        assert!(!output.skipped);
    }

    #[test]
    fn indexes_unreadable_file_with_hash_and_no_thumb() {
        let dir = tempdir().unwrap();
        let broken = dir.path().join("broken.jpg");
        std::fs::write(&broken, b"not-a-real-image").unwrap();

        let ctx = MetadataContext::in_place(broken.clone());
        let output = index_file(7, &broken, dir.path(), &ctx, 0, "image", None, None, None);
        assert_eq!(output.meta.asset_id, 7);
        assert!(output.meta.capture_at.is_none());
        assert!(output.raw_tags.is_empty());
        assert!(output.content_hash.is_some());
        assert!(output.thumb_key.is_none());
        assert!(!output.skipped);
    }

    #[test]
    fn index_file_skips_when_thumb_mtime_and_hash_match() {
        let dir = tempdir().unwrap();
        let photo = dir.path().join("sample.jpg");
        let bytes = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(&photo, bytes).unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let ctx = MetadataContext::in_place(photo.clone());
        let hash = sha256_bytes(bytes);
        let first = index_file(42, &photo, &thumb_dir, &ctx, 99, "image", None, None, None);
        assert!(!first.skipped);
        let second = index_file(
            42,
            &photo,
            &thumb_dir,
            &ctx,
            99,
            "image",
            first.thumb_key.as_deref(),
            Some(99),
            Some(&hash),
        );
        assert!(second.skipped);
        assert_eq!(second.content_hash.as_deref(), Some(hash.as_str()));
    }

    #[test]
    fn index_batch_on_disk_processes_multiple_assets() {
        let dir = tempdir().unwrap();
        let photo_a = dir.path().join("a.jpg");
        let photo_b = dir.path().join("b.jpg");
        std::fs::write(&photo_a, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        std::fs::write(&photo_b, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let items = vec![
            BatchIndexItem {
                asset_id: 1,
                path: photo_a.clone(),
                metadata_ctx: MetadataContext::in_place(photo_a),
                mtime_ns: 0,
                kind: "image".into(),
                prior_thumb_key: None,
                prior_indexed_mtime_ns: None,
                prior_content_hash: None,
            },
            BatchIndexItem {
                asset_id: 2,
                path: photo_b.clone(),
                metadata_ctx: MetadataContext::in_place(photo_b),
                mtime_ns: 0,
                kind: "image".into(),
                prior_thumb_key: None,
                prior_indexed_mtime_ns: None,
                prior_content_hash: None,
            },
        ];
        let indexed = index_batch_on_disk(&items, &thumb_dir, IoProfile::Local);
        assert_eq!(indexed.len(), 2);
        assert!(indexed
            .iter()
            .all(|(_, output)| output.thumb_key.is_some() && output.content_hash.is_some()));
    }

    #[test]
    fn network_profile_limits_parallelism() {
        assert_eq!(index_parallelism(IoProfile::Network), 2);
    }
}
