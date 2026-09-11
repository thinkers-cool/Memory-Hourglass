use crate::catalog::models::{AssetMeta, RawTag};
use crate::metadata::{MetadataContext, MetadataService};
use crate::thumb::ThumbService;
use std::path::Path;

#[derive(Debug)]
pub struct IndexOutput {
    pub meta: AssetMeta,
    pub raw_tags: Vec<RawTag>,
    pub thumb_key: Option<String>,
}

pub fn index_asset_on_disk(
    asset_id: i64,
    path: &Path,
    thumb_dir: &Path,
    metadata_ctx: &MetadataContext,
) -> IndexOutput {
    if std::env::var_os("MEMHG_TEST_INDEX_ON_DISK_PANIC").is_some() {
        std::env::remove_var("MEMHG_TEST_INDEX_ON_DISK_PANIC");
        panic!("index on disk panic");
    }
    let thumb_key = ThumbService::new(thumb_dir.to_path_buf())
        .ensure_thumbnail(asset_id, path)
        .ok();

    let (read, raw_tags) = match MetadataService::read_meta(metadata_ctx) {
        Ok((read, raw_tags)) => (read, raw_tags),
        Err(_) => (
            AssetMeta {
                asset_id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: None,
                latitude: None,
                longitude: None,
                keywords_json: None,
            },
            Vec::new(),
        ),
    };

    let meta = AssetMeta {
        asset_id,
        capture_at: read.capture_at,
        camera: read.camera,
        lens: read.lens,
        rating: read.rating,
        latitude: read.latitude,
        longitude: read.longitude,
        keywords_json: read.keywords_json,
    };

    IndexOutput {
        meta,
        raw_tags,
        thumb_key,
    }
}

pub fn resolved_thumb_key<'a>(
    _kind: &str,
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
            resolved_thumb_key("image", Some("new.webp"), Some("old.webp")),
            Some("new.webp")
        );
    }

    #[test]
    fn resolved_thumb_key_falls_back_to_prior_value() {
        assert_eq!(
            resolved_thumb_key("video", None, Some("old.webp")),
            Some("old.webp")
        );
        assert_eq!(resolved_thumb_key("raw", None, None), None);
    }

    #[test]
    fn indexes_jpeg_metadata_and_thumbnail() {
        let dir = tempdir().unwrap();
        let photo = dir.path().join("sample.jpg");
        std::fs::write(&photo, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();

        let thumb_dir = dir.path().join("thumbs");
        let ctx = MetadataContext::in_place(photo.clone());
        let output = index_asset_on_disk(42, &photo, &thumb_dir, &ctx);
        assert_eq!(output.meta.asset_id, 42);
        assert_eq!(output.thumb_key.as_deref(), Some("42.webp"));
        assert!(thumb_dir.join("42.webp").exists());
    }

    #[test]
    fn indexes_unreadable_file_with_empty_metadata() {
        let dir = tempdir().unwrap();
        let broken = dir.path().join("broken.jpg");
        std::fs::write(&broken, b"not-a-real-image").unwrap();

        let ctx = MetadataContext::in_place(broken.clone());
        let output = index_asset_on_disk(7, &broken, dir.path(), &ctx);
        assert_eq!(output.meta.asset_id, 7);
        assert!(output.meta.capture_at.is_none());
        assert!(output.raw_tags.is_empty());
    }
}
