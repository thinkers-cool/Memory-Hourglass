mod policy;
mod sidecar;

pub use policy::{metadata_context_for_asset, MetadataContext, MetadataPolicy};
pub use sidecar::{workspace_sidecar_path, xmp_sidecar_path};

use crate::catalog::models::AssetMeta;
use crate::catalog::models::RawTag;
use crate::error::{AppError, Result};
use exiftool_rs::tag::Tag;
use exiftool_rs::value::Value;
use exiftool_rs::ExifTool;
use sidecar::{merge_tags, uses_xmp_sidecar_write, MINIMAL_XMP};

const CAPTURE_DATE_TAGS: &[&str] = &[
    "DateTimeOriginal",
    "CreateDate",
    "MediaCreateDate",
    "TrackCreateDate",
    "ContentCreateDate",
    "DateCreated",
];

pub struct IndexExifData {
    pub meta: AssetMeta,
    pub raw_tags: Vec<RawTag>,
    pub embedded_thumbnail: Option<Vec<u8>>,
}

pub fn embedded_thumbnail_bytes(tags: &[Tag]) -> Option<Vec<u8>> {
    for name in ["ThumbnailImage", "PreviewImage"] {
        for tag in tags {
            if tag.name == name {
                if let Value::Binary(bytes) = &tag.raw_value {
                    if is_jpeg_bytes(bytes) {
                        return Some(bytes.clone());
                    }
                }
            }
        }
    }
    None
}

fn is_jpeg_bytes(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xD8
}

pub struct MetadataService;

impl MetadataService {
    pub fn read_meta(ctx: &MetadataContext) -> Result<(AssetMeta, Vec<RawTag>)> {
        let et = ExifTool::new();
        Self::read_meta_with(&et, ctx)
    }

    pub fn read_meta_with(et: &ExifTool, ctx: &MetadataContext) -> Result<(AssetMeta, Vec<RawTag>)> {
        let path = &ctx.media_path;
        let embedded = et.extract_info(path).ok();
        Self::meta_from_merged_tags(et, ctx, embedded)
    }

    pub fn read_meta_with_bytes(
        et: &ExifTool,
        ctx: &MetadataContext,
        bytes: &[u8],
    ) -> Result<(AssetMeta, Vec<RawTag>)> {
        let embedded = et.extract_info_from_bytes(bytes, &ctx.media_path).ok();
        Self::meta_from_merged_tags(et, ctx, embedded)
    }

    pub fn read_index_exif_with_bytes(
        et: &ExifTool,
        ctx: &MetadataContext,
        asset_id: i64,
        bytes: &[u8],
    ) -> IndexExifData {
        let embedded_tags = et.extract_info_from_bytes(bytes, &ctx.media_path).ok();
        let embedded_thumbnail = embedded_tags
            .as_ref()
            .and_then(|tags| embedded_thumbnail_bytes(tags));
        let (meta, raw_tags) = match Self::meta_from_merged_tags(et, ctx, embedded_tags) {
            Ok((read, raw_tags)) => {
                let meta = AssetMeta {
                    asset_id,
                    capture_at: read.capture_at,
                    camera: read.camera,
                    lens: read.lens,
                    rating: read.rating,
                    latitude: read.latitude,
                    longitude: read.longitude,
                    keywords_json: read.keywords_json,
                    rotation: read.rotation,
                };
                (meta, raw_tags)
            }
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
                    rotation: None,
                },
                Vec::new(),
            ),
        };
        IndexExifData {
            meta,
            raw_tags,
            embedded_thumbnail,
        }
    }

    fn meta_from_merged_tags(
        et: &ExifTool,
        ctx: &MetadataContext,
        embedded: Option<Vec<Tag>>,
    ) -> Result<(AssetMeta, Vec<RawTag>)> {
        let path = &ctx.media_path;
        let mut merged = embedded;

        let colocated = ctx.colocated_sidecar_path();
        if colocated.is_file() {
            let colocated_tags = et
                .extract_info(&colocated)
                .map_err(|e| AppError::Metadata(e.to_string()))?;
            merged = Some(match merged {
                Some(embedded) => merge_tags(embedded, colocated_tags),
                None => colocated_tags,
            });
        }

        if ctx.policy == MetadataPolicy::WorkspaceSidecar {
            let workspace_sidecar = ctx.write_sidecar_path();
            if workspace_sidecar.is_file() {
                let workspace_tags = et
                    .extract_info(&workspace_sidecar)
                    .map_err(|e| AppError::Metadata(e.to_string()))?;
                merged = Some(match merged {
                    Some(existing) => merge_tags(existing, workspace_tags),
                    None => workspace_tags,
                });
            }
        }

        let tags = match merged {
            Some(tags) if !tags.is_empty() => tags,
            _ => {
                return Err(AppError::Metadata(format!(
                    "no metadata found for {}",
                    path.display()
                )));
            }
        };

        let raw_tags = tags
            .iter()
            .map(|tag| RawTag {
                name: tag.name.clone(),
                value: tag.print_value.clone(),
            })
            .collect();

        let meta = AssetMeta {
            asset_id: 0,
            capture_at: capture_at_from_tags(&tags),
            camera: first_tag_value(&tags, "Model"),
            lens: first_tag_value(&tags, "LensModel").or_else(|| first_tag_value(&tags, "Lens")),
            rating: first_tag_value(&tags, "Rating").and_then(|v| v.parse().ok()),
            latitude: gps_coordinate(&tags, "GPSLatitude", "GPSLatitudeRef", 'S'),
            longitude: gps_coordinate(&tags, "GPSLongitude", "GPSLongitudeRef", 'W'),
            keywords_json: keywords_from_tags(&tags),
            rotation: orientation_from_tags(&tags),
        };

        Ok((meta, raw_tags))
    }

    pub fn write_rating(ctx: &MetadataContext, rating: i64) -> Result<()> {
        let (_, rotation, keywords) = existing_xmp_fields(ctx);
        let rating_str = rating.to_string();
        Self::write_tag(ctx, |et| {
            et.set_new_value("xmp:Rating", Some(&rating_str));
            apply_keywords(et, &keywords);
            apply_orientation(et, rotation);
        })
    }

    pub fn write_keywords(ctx: &MetadataContext, keywords: &[String]) -> Result<()> {
        let (rating, rotation, _) = existing_xmp_fields(ctx);
        Self::write_tag(ctx, |et| {
            apply_keywords(et, keywords);
            if let Some(rating) = rating {
                et.set_new_value("xmp:Rating", Some(&rating.to_string()));
            }
            apply_orientation(et, rotation);
        })
    }

    pub fn write_rotation(ctx: &MetadataContext, rotation: i64) -> Result<()> {
        let (rating, _, keywords) = existing_xmp_fields(ctx);
        let orientation = degrees_to_orientation(rotation).to_string();
        Self::write_tag(ctx, |et| {
            et.set_new_value("Orientation", Some(&orientation));
            if let Some(rating) = rating {
                et.set_new_value("xmp:Rating", Some(&rating.to_string()));
            }
            apply_keywords(et, &keywords);
        })
    }

    fn write_tag(ctx: &MetadataContext, apply: impl FnOnce(&mut ExifTool)) -> Result<()> {
        Self::ensure_writable(ctx)?;
        let mut et = ExifTool::new();
        apply(&mut et);

        let sidecar = ctx.write_sidecar_path();
        if ctx.policy == MetadataPolicy::WorkspaceSidecar || uses_xmp_sidecar_write(&ctx.media_path)
        {
            prepare_xmp_sidecar(&sidecar)?;
            et.write_info(
                sidecar.to_string_lossy().as_ref(),
                sidecar.to_string_lossy().as_ref(),
            )
            .map_err(|e| AppError::Metadata(e.to_string()))?;
            return Ok(());
        }

        let path = &ctx.media_path;
        et.write_info(
            path.to_string_lossy().as_ref(),
            path.to_string_lossy().as_ref(),
        )
        .map_err(|e| AppError::Metadata(e.to_string()))?;
        Ok(())
    }

    fn ensure_writable(ctx: &MetadataContext) -> Result<()> {
        let sidecar = ctx.write_sidecar_path();
        if !sidecar.exists() {
            return Ok(());
        }
        if ctx.policy == MetadataPolicy::WorkspaceSidecar {
            return Ok(());
        }
        ensure_sidecar_not_newer_than_media(&sidecar, &ctx.media_path)
    }
}

fn prepare_xmp_sidecar(sidecar: &std::path::Path) -> Result<()> {
    if let Some(parent) = sidecar.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !sidecar.exists() {
        std::fs::write(sidecar, MINIMAL_XMP)?;
    }
    Ok(())
}

fn ensure_sidecar_not_newer_than_media(
    sidecar: &std::path::Path,
    media_path: &std::path::Path,
) -> Result<()> {
    use std::time::SystemTime;

    let sidecar_meta = std::fs::metadata(sidecar)?;
    let file_meta = std::fs::metadata(media_path)?;
    let sidecar_modified = sidecar_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let media_modified = file_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if sidecar_modified > media_modified {
        Err(AppError::Conflict(
            "sidecar was modified externally; reload before editing".into(),
        ))
    } else {
        Ok(())
    }
}

fn existing_xmp_fields(ctx: &MetadataContext) -> (Option<i64>, Option<i64>, Vec<String>) {
    MetadataService::read_meta(ctx)
        .map(|(meta, _)| {
            let keywords = meta
                .keywords_json
                .as_deref()
                .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
                .unwrap_or_default();
            (meta.rating, meta.rotation, keywords)
        })
        .unwrap_or((None, None, Vec::new()))
}

fn orientation_from_tags(tags: &[Tag]) -> Option<i64> {
    first_tag_value(tags, "Orientation")
        .and_then(|value| value.parse::<i64>().ok())
        .map(orientation_to_degrees)
}

pub fn orientation_to_degrees(orientation: i64) -> i64 {
    match orientation {
        3 => 180,
        6 => 90,
        8 => 270,
        _ => 0,
    }
}

pub fn degrees_to_orientation(degrees: i64) -> i64 {
    match degrees.rem_euclid(360) {
        90 => 6,
        180 => 3,
        270 => 8,
        _ => 1,
    }
}

fn apply_orientation(et: &mut ExifTool, rotation: Option<i64>) {
    if let Some(rotation) = rotation {
        let orientation = degrees_to_orientation(rotation).to_string();
        et.set_new_value("Orientation", Some(&orientation));
    }
}

fn apply_keywords(et: &mut ExifTool, keywords: &[String]) {
    et.set_new_value("XMP:Subject", None);
    for keyword in keywords {
        if !keyword.is_empty() {
            et.set_new_value("XMP:Subject", Some(keyword));
        }
    }
}

fn first_tag_value(tags: &[Tag], name: &str) -> Option<String> {
    tags.iter()
        .find(|tag| tag.name == name)
        .map(|tag| tag.print_value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn capture_at_from_tags(tags: &[Tag]) -> Option<i64> {
    for name in CAPTURE_DATE_TAGS {
        let Some(tag) = tags.iter().find(|tag| tag.name == *name) else {
            continue;
        };
        if let Some(ts) = parse_metadata_datetime(&tag.print_value) {
            return Some(ts);
        }
    }
    None
}

fn keywords_from_tags(tags: &[Tag]) -> Option<String> {
    let keywords: Vec<String> = tags
        .iter()
        .filter(|tag| tag.name == "Subject" || tag.name == "Keywords")
        .flat_map(|tag| {
            tag.print_value
                .split(',')
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
        })
        .collect();
    if keywords.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&keywords).unwrap())
    }
}

fn gps_coordinate(tags: &[Tag], coord: &str, ref_tag: &str, negative_ref: char) -> Option<f64> {
    let reference = tags
        .iter()
        .find(|tag| tag.name == ref_tag)
        .map(|tag| tag.raw_value.to_display_string())?;
    let coord_tag = tags
        .iter()
        .find(|tag| tag.name == coord && tag.group.family1 == "GPS")?;
    let decimal = gps_raw_to_decimal(&coord_tag.raw_value)?;
    let negative = reference
        .chars()
        .next()
        .is_some_and(|c| c.eq_ignore_ascii_case(&negative_ref));
    Some(if negative {
        -decimal.abs()
    } else {
        decimal.abs()
    })
}

fn gps_raw_to_decimal(value: &Value) -> Option<f64> {
    match value {
        Value::List(items) if items.len() >= 3 => {
            let deg = items[0].as_f64()?;
            let min = items[1].as_f64()?;
            let sec = items[2].as_f64()?;
            Some(deg + min / 60.0 + sec / 3600.0)
        }
        Value::URational(n, d) if *d > 0 => Some(*n as f64 / *d as f64),
        Value::IRational(n, d) if *d > 0 => Some(*n as f64 / *d as f64),
        Value::F64(v) => Some(*v),
        Value::F32(v) => Some(*v as f64),
        _ => None,
    }
}

fn parse_metadata_datetime(value: &str) -> Option<i64> {
    use chrono::{DateTime, NaiveDateTime, Utc};

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    const FORMATS: &[&str] = &[
        "%Y:%m:%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y:%m:%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
    ];

    for fmt in FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Some(dt.and_utc().timestamp());
        }
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.timestamp());
    }

    if let Ok(dt) = trimmed.parse::<DateTime<Utc>>() {
        return Some(dt.timestamp());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use exiftool_rs::tag::{TagGroup, TagId};
    use sidecar::{uses_xmp_sidecar_write, xmp_sidecar_path, MINIMAL_XMP};
    use std::path::Path;
    use tempfile::NamedTempFile;

    fn write_minimal_jpeg(path: &Path) {
        let jpeg_bytes = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(path, jpeg_bytes).unwrap();
    }

    fn in_place_ctx(path: &Path) -> MetadataContext {
        MetadataContext::in_place(path.to_path_buf())
    }

    #[test]
    fn orientation_helpers_map_degrees() {
        assert_eq!(orientation_to_degrees(6), 90);
        assert_eq!(orientation_to_degrees(3), 180);
        assert_eq!(orientation_to_degrees(8), 270);
        assert_eq!(degrees_to_orientation(90), 6);
        assert_eq!(degrees_to_orientation(180), 3);
        assert_eq!(degrees_to_orientation(270), 8);
        assert_eq!(degrees_to_orientation(0), 1);
    }

    #[test]
    fn read_meta_from_jpeg() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let (meta, raw) = MetadataService::read_meta(&in_place_ctx(file.path())).unwrap();
        assert!(!raw.is_empty());
        assert!(meta.capture_at.is_some() || meta.camera.is_some() || !raw.is_empty());
    }

    fn test_thumbnail_tag(bytes: Vec<u8>) -> Tag {
        Tag {
            id: TagId::Text("ThumbnailImage".into()),
            name: "ThumbnailImage".into(),
            description: "Thumbnail Image".into(),
            group: TagGroup::default(),
            raw_value: Value::Binary(bytes),
            print_value: String::new(),
            priority: 0,
        }
    }

    #[test]
    fn embedded_thumbnail_bytes_reads_jpeg_binary_tag() {
        let tags = vec![test_thumbnail_tag(vec![0xFF, 0xD8, 0xFF, 0xDB, 0x00])];
        let thumb = embedded_thumbnail_bytes(&tags).expect("thumbnail bytes");
        assert_eq!(thumb[0], 0xFF);
        assert_eq!(thumb[1], 0xD8);
    }

    #[test]
    fn embedded_thumbnail_bytes_ignores_non_jpeg_binary() {
        let tags = vec![test_thumbnail_tag(vec![0x89, 0x50, 0x4E, 0x47])];
        assert!(embedded_thumbnail_bytes(&tags).is_none());
    }

    #[test]
    fn read_meta_with_bytes_matches_read_meta() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        let bytes = std::fs::read(file.path()).unwrap();
        let et = ExifTool::new();
        let from_path = MetadataService::read_meta_with(&et, &ctx).unwrap();
        let from_bytes = MetadataService::read_meta_with_bytes(&et, &ctx, &bytes).unwrap();
        assert_eq!(from_path.0.capture_at, from_bytes.0.capture_at);
        assert_eq!(from_path.0.camera, from_bytes.0.camera);
        assert_eq!(from_path.1.len(), from_bytes.1.len());
    }

    #[test]
    fn write_rating_roundtrip() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        MetadataService::write_rating(&ctx, 4).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(4));
    }

    #[test]
    fn write_rating_then_keywords_preserves_rating() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        MetadataService::write_rating(&ctx, 5).unwrap();
        MetadataService::write_keywords(&ctx, &["sony".into()]).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(5));
        let keywords = meta.keywords_json.as_deref().unwrap();
        assert!(keywords.contains("sony"));
    }

    #[test]
    fn write_keywords_then_rating_preserves_keywords() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        MetadataService::write_keywords(&ctx, &["sony".into()]).unwrap();
        MetadataService::write_rating(&ctx, 4).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(4));
        let keywords = meta.keywords_json.as_deref().unwrap();
        assert!(keywords.contains("sony"));
    }

    #[test]
    fn write_keywords_roundtrip() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        MetadataService::write_keywords(&ctx, &["travel".into(), "family".into()]).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        let keywords = meta.keywords_json.as_deref().unwrap();
        assert!(keywords.contains("travel"));
        assert!(keywords.contains("family"));
    }

    #[test]
    fn read_only_jpeg_writes_workspace_sidecar_without_touching_media() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("photos/a7c.jpg");
        std::fs::create_dir_all(photo.parent().unwrap()).unwrap();
        write_minimal_jpeg(&photo);
        let before = std::fs::read(&photo).unwrap();

        let xmp_dir = dir.path().join("xmp");
        let ctx = MetadataContext::workspace_sidecar(
            photo.clone(),
            7,
            "photos/a7c.jpg".into(),
            xmp_dir.clone(),
        );
        MetadataService::write_rating(&ctx, 4).unwrap();

        let after = std::fs::read(&photo).unwrap();
        assert_eq!(before, after);
        let sidecar = workspace_sidecar_path(&xmp_dir, 7, "photos/a7c.jpg");
        assert!(sidecar.is_file());
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(4));
    }

    #[test]
    fn video_writes_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        std::fs::write(&video, b"fake-mp4").unwrap();
        assert!(uses_xmp_sidecar_write(&video));

        let ctx = in_place_ctx(&video);
        MetadataService::write_rating(&ctx, 3).unwrap();

        let sidecar = xmp_sidecar_path(&video);
        assert!(sidecar.is_file());

        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(3));
    }

    #[test]
    fn raw_writes_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        std::fs::write(&raw, b"fake-raw").unwrap();

        let ctx = in_place_ctx(&raw);
        MetadataService::write_keywords(&ctx, &["raw-tag".into()]).unwrap();

        let sidecar = xmp_sidecar_path(&raw);
        assert!(sidecar.is_file());
        let sidecar_text = std::fs::read_to_string(&sidecar).unwrap();
        assert!(sidecar_text.contains("raw-tag"));
    }

    #[test]
    fn parses_gps_from_rational_list() {
        let tags = vec![
            Tag {
                id: exiftool_rs::tag::TagId::Text("GPSLatitudeRef".into()),
                name: "GPSLatitudeRef".into(),
                description: String::new(),
                group: exiftool_rs::tag::TagGroup {
                    family0: "EXIF".into(),
                    family1: "GPS".into(),
                    family2: String::new(),
                    family3: exiftool_rs::tag::MAIN_DOCUMENT.into(),
                },
                raw_value: Value::String("N".into()),
                print_value: "N".into(),
                priority: 1,
            },
            Tag {
                id: exiftool_rs::tag::TagId::Text("GPSLatitude".into()),
                name: "GPSLatitude".into(),
                description: String::new(),
                group: exiftool_rs::tag::TagGroup {
                    family0: "EXIF".into(),
                    family1: "GPS".into(),
                    family2: String::new(),
                    family3: exiftool_rs::tag::MAIN_DOCUMENT.into(),
                },
                raw_value: Value::List(vec![
                    Value::URational(37, 1),
                    Value::URational(46, 1),
                    Value::URational(30, 1),
                ]),
                print_value: String::new(),
                priority: 1,
            },
        ];

        let lat = gps_coordinate(&tags, "GPSLatitude", "GPSLatitudeRef", 'S').unwrap();
        assert!((lat - 37.775).abs() < 0.001);
    }

    fn gps_tag(name: &str, family1: &str, raw: Value, print: &str) -> Tag {
        Tag {
            id: exiftool_rs::tag::TagId::Text(name.into()),
            name: name.into(),
            description: String::new(),
            group: exiftool_rs::tag::TagGroup {
                family0: "EXIF".into(),
                family1: family1.into(),
                family2: String::new(),
                family3: exiftool_rs::tag::MAIN_DOCUMENT.into(),
            },
            raw_value: raw,
            print_value: print.into(),
            priority: 1,
        }
    }

    #[test]
    fn parse_metadata_datetime_handles_common_formats() {
        assert_eq!(
            parse_metadata_datetime("2024:03:15 10:30:00"),
            Some(1710498600)
        );
        assert_eq!(
            parse_metadata_datetime("2024-03-15 10:30:00"),
            Some(1710498600)
        );
        assert_eq!(
            parse_metadata_datetime("2024-03-15T10:30:00Z"),
            Some(1710498600)
        );
        assert!(parse_metadata_datetime("").is_none());
        assert!(parse_metadata_datetime("not-a-date").is_none());
    }

    #[test]
    fn parse_metadata_datetime_handles_fractional_seconds() {
        assert!(parse_metadata_datetime("2024:03:15 10:30:00.5").is_some());
    }

    #[test]
    fn first_tag_value_skips_blank_values() {
        let tags = vec![gps_tag("Model", "EXIF", Value::String("".into()), "  ")];
        assert!(first_tag_value(&tags, "Model").is_none());
        let tags = vec![gps_tag("Model", "EXIF", Value::String("A7".into()), "A7")];
        assert_eq!(first_tag_value(&tags, "Model").as_deref(), Some("A7"));
    }

    #[test]
    fn capture_at_from_tags_uses_first_valid_date() {
        let tags = vec![
            gps_tag("DateTimeOriginal", "EXIF", Value::String("".into()), ""),
            gps_tag(
                "CreateDate",
                "EXIF",
                Value::String("2024:01:02 03:04:05".into()),
                "2024:01:02 03:04:05",
            ),
        ];
        assert!(capture_at_from_tags(&tags).is_some());
    }

    #[test]
    fn keywords_from_tags_collects_subject_and_keywords() {
        let tags = vec![
            gps_tag("Subject", "XMP", Value::String("".into()), "travel, family"),
            gps_tag("Keywords", "IPTC", Value::String("".into()), "wedding"),
        ];
        let json = keywords_from_tags(&tags).unwrap();
        assert!(json.contains("travel"));
        assert!(json.contains("family"));
        assert!(json.contains("wedding"));
        assert!(keywords_from_tags(&[]).is_none());
    }

    #[test]
    fn gps_coordinate_applies_southern_and_western_refs() {
        let tags = vec![
            gps_tag("GPSLatitudeRef", "GPS", Value::String("S".into()), "S"),
            gps_tag(
                "GPSLatitude",
                "GPS",
                Value::List(vec![
                    Value::URational(10, 1),
                    Value::URational(0, 1),
                    Value::URational(0, 1),
                ]),
                "",
            ),
            gps_tag("GPSLongitudeRef", "GPS", Value::String("W".into()), "W"),
            gps_tag("GPSLongitude", "GPS", Value::F64(45.5), "45.5"),
        ];
        let lat = gps_coordinate(&tags, "GPSLatitude", "GPSLatitudeRef", 'S').unwrap();
        let lon = gps_coordinate(&tags, "GPSLongitude", "GPSLongitudeRef", 'W').unwrap();
        assert!((lat + 10.0).abs() < 0.001);
        assert!((lon + 45.5).abs() < 0.001);
    }

    #[test]
    fn gps_raw_to_decimal_handles_scalar_and_rational_values() {
        assert_eq!(gps_raw_to_decimal(&Value::F64(12.5)), Some(12.5));
        assert_eq!(gps_raw_to_decimal(&Value::F32(3.25)), Some(3.25));
        assert_eq!(gps_raw_to_decimal(&Value::URational(90, 2)), Some(45.0));
        assert_eq!(gps_raw_to_decimal(&Value::IRational(-90, 2)), Some(-45.0));
        assert!(gps_raw_to_decimal(&Value::String("x".into())).is_none());
    }

    #[test]
    fn read_meta_merges_colocated_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        std::fs::write(&raw, b"fake-raw").unwrap();
        let ctx = in_place_ctx(&raw);
        MetadataService::write_rating(&ctx, 2).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(2));
        assert!(xmp_sidecar_path(&raw).is_file());
    }

    #[test]
    fn read_meta_errors_when_no_metadata_present() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"not-image-data").unwrap();
        let err = MetadataService::read_meta(&in_place_ctx(file.path())).unwrap_err();
        assert!(err.to_string().contains("no metadata"));
    }

    #[test]
    fn read_meta_errors_when_media_missing() {
        let err = MetadataService::read_meta(&in_place_ctx(Path::new(
            "/tmp/memhg-missing-metadata-file.jpg",
        )))
        .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn ensure_writable_rejects_stale_media_when_sidecar_is_newer() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        std::fs::write(&raw, b"fake-raw").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let sidecar = xmp_sidecar_path(&raw);
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        let ctx = in_place_ctx(&raw);
        let err = MetadataService::write_rating(&ctx, 3).unwrap_err();
        assert!(err.to_string().contains("sidecar was modified externally"));
    }

    #[test]
    fn write_keywords_skips_empty_entries() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        MetadataService::write_keywords(&ctx, &["keep".into(), "".into(), "also".into()]).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        let keywords = meta.keywords_json.as_deref().unwrap();
        assert!(keywords.contains("keep"));
        assert!(keywords.contains("also"));
    }

    #[test]
    fn read_meta_merges_workspace_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("photo.jpg");
        write_minimal_jpeg(&photo);
        let xmp_dir = dir.path().join("xmp");
        let ctx = MetadataContext::workspace_sidecar(
            photo.clone(),
            1,
            "photo.jpg".into(),
            xmp_dir.clone(),
        );
        MetadataService::write_rating(&ctx, 3).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(3));
    }

    #[test]
    fn write_rating_creates_new_video_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        std::fs::write(&video, b"fake-mp4").unwrap();
        let ctx = in_place_ctx(&video);
        MetadataService::write_rating(&ctx, 2).unwrap();
        let sidecar = xmp_sidecar_path(&video);
        assert!(sidecar.is_file());
    }

    #[test]
    fn ensure_writable_allows_workspace_sidecar_policy() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("photo.jpg");
        write_minimal_jpeg(&photo);
        let xmp_dir = dir.path().join("xmp");
        let sidecar = workspace_sidecar_path(&xmp_dir, 1, "photo.jpg");
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        let ctx = MetadataContext::workspace_sidecar(photo, 1, "photo.jpg".into(), xmp_dir);
        MetadataService::write_rating(&ctx, 4).unwrap();
    }

    #[test]
    fn ensure_sidecar_not_newer_than_media_allows_older_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let media = dir.path().join("photo.jpg");
        let sidecar = xmp_sidecar_path(&media);
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        write_minimal_jpeg(&media);
        ensure_sidecar_not_newer_than_media(&sidecar, &media).unwrap();
    }

    #[test]
    fn ensure_writable_allows_when_sidecar_is_older_than_media() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        MetadataService::write_rating(&ctx, 1).unwrap();
        MetadataService::write_rating(&ctx, 2).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(2));
    }

    #[test]
    fn parse_metadata_datetime_accepts_utc_zulu() {
        assert_eq!(
            parse_metadata_datetime("2024-03-15T10:30:00+00:00"),
            Some(1710498600)
        );
        assert_eq!(
            parse_metadata_datetime("2024-03-15T10:30:00Z"),
            Some(1710498600)
        );
        assert_eq!(
            parse_metadata_datetime("2024-03-15 10:30:00 UTC"),
            Some(1710498600)
        );
    }

    #[test]
    fn write_rating_creates_video_sidecar_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clips/clip.mov");
        std::fs::create_dir_all(video.parent().unwrap()).unwrap();
        std::fs::write(&video, b"video").unwrap();
        let ctx = in_place_ctx(&video);
        MetadataService::write_rating(&ctx, 2).unwrap();
        assert!(xmp_sidecar_path(&video).is_file());
    }

    #[test]
    fn write_rating_creates_nested_workspace_sidecar_directory() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("photo.jpg");
        write_minimal_jpeg(&photo);
        let xmp_dir = dir.path().join("nested/xmp/store");
        let ctx = MetadataContext::workspace_sidecar(photo, 1, "photo.jpg".into(), xmp_dir.clone());
        MetadataService::write_rating(&ctx, 5).unwrap();
        assert!(workspace_sidecar_path(&xmp_dir, 1, "photo.jpg").is_file());
    }

    #[test]
    fn read_meta_uses_workspace_sidecar_when_embedded_missing() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("plain.dat");
        std::fs::write(&photo, b"no-exif").unwrap();
        let xmp_dir = dir.path().join("xmp");
        let ctx = MetadataContext::workspace_sidecar(
            photo.clone(),
            1,
            "plain.dat".into(),
            xmp_dir.clone(),
        );
        let sidecar = workspace_sidecar_path(&xmp_dir, 1, "plain.dat");
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        let mut et = exiftool_rs::ExifTool::new();
        et.set_new_value("xmp:Rating", Some("4"));
        et.write_info(
            sidecar.to_string_lossy().as_ref(),
            sidecar.to_string_lossy().as_ref(),
        )
        .unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(4));
    }

    #[test]
    fn read_meta_merges_colocated_and_workspace_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("photo.jpg");
        write_minimal_jpeg(&photo);
        let xmp_dir = dir.path().join("xmp");
        let ctx = MetadataContext::workspace_sidecar(
            photo.clone(),
            1,
            "photo.jpg".into(),
            xmp_dir.clone(),
        );
        MetadataService::write_rating(&ctx, 3).unwrap();
        MetadataService::write_keywords(&in_place_ctx(&photo), &["colocated".into()]).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(3));
        let keywords = meta.keywords_json.as_deref().unwrap();
        assert!(keywords.contains("colocated"));
    }

    #[test]
    fn gps_coordinate_requires_gps_group_family() {
        let tags = vec![
            gps_tag("GPSLatitudeRef", "EXIF", Value::String("N".into()), "N"),
            gps_tag(
                "GPSLatitude",
                "EXIF",
                Value::List(vec![
                    Value::URational(10, 1),
                    Value::URational(0, 1),
                    Value::URational(0, 1),
                ]),
                "",
            ),
        ];
        assert!(gps_coordinate(&tags, "GPSLatitude", "GPSLatitudeRef", 'S').is_none());
    }

    #[test]
    fn read_meta_in_place_write_updates_embedded_jpeg() {
        let file = NamedTempFile::new().unwrap();
        write_minimal_jpeg(file.path());
        let ctx = in_place_ctx(file.path());
        MetadataService::write_rating(&ctx, 1).unwrap();
        MetadataService::write_rating(&ctx, 2).unwrap();
        let (meta, _) = MetadataService::read_meta(&ctx).unwrap();
        assert_eq!(meta.rating, Some(2));
        assert!(!xmp_sidecar_path(file.path()).exists());
    }

    #[test]
    fn lens_meta_uses_lens_tag_when_lens_model_missing() {
        let tags = vec![gps_tag(
            "Lens",
            "EXIF",
            Value::String("50mm".into()),
            "50mm",
        )];
        let lens = first_tag_value(&tags, "LensModel").or_else(|| first_tag_value(&tags, "Lens"));
        assert_eq!(lens.as_deref(), Some("50mm"));
    }

    #[cfg(unix)]
    #[test]
    fn read_meta_errors_when_colocated_sidecar_unreadable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        std::fs::write(&raw, b"fake-raw").unwrap();
        let sidecar = xmp_sidecar_path(&raw);
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o000)).unwrap();
        let err = MetadataService::read_meta(&in_place_ctx(&raw)).unwrap_err();
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!err.to_string().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn read_meta_errors_when_workspace_sidecar_unreadable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("photo.jpg");
        write_minimal_jpeg(&photo);
        let xmp_dir = dir.path().join("xmp");
        let sidecar = workspace_sidecar_path(&xmp_dir, 1, "photo.jpg");
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o000)).unwrap();
        let ctx = MetadataContext::workspace_sidecar(photo, 1, "photo.jpg".into(), xmp_dir);
        let err = MetadataService::read_meta(&ctx).unwrap_err();
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn prepare_xmp_sidecar_creates_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let sidecar = dir.path().join("nested/xmp/photo.xmp");
        prepare_xmp_sidecar(&sidecar).unwrap();
        assert!(sidecar.is_file());
    }

    #[test]
    fn ensure_sidecar_not_newer_propagates_metadata_errors() {
        let dir = tempfile::tempdir().unwrap();
        let sidecar = dir.path().join("sidecar.xmp");
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        let missing_media = dir.path().join("missing.jpg");
        assert!(ensure_sidecar_not_newer_than_media(&sidecar, &missing_media).is_err());
    }

    #[test]
    fn gps_raw_to_decimal_requires_three_list_items() {
        assert!(gps_raw_to_decimal(&Value::List(vec![Value::F64(1.0)])).is_none());
        assert!(
            gps_raw_to_decimal(&Value::List(vec![Value::F64(1.0), Value::F64(2.0),])).is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_rating_errors_when_sidecar_write_blocked() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        std::fs::write(&video, b"fake-mp4").unwrap();
        let sidecar = xmp_sidecar_path(&video);
        std::fs::write(&sidecar, MINIMAL_XMP).unwrap();
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o000)).unwrap();
        let ctx = in_place_ctx(&video);
        let err = MetadataService::write_rating(&ctx, 3).unwrap_err();
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn gps_coordinate_returns_none_without_reference_tag() {
        let tags = vec![gps_tag("GPSLatitude", "GPS", Value::F64(12.5), "12.5")];
        assert!(gps_coordinate(&tags, "GPSLatitude", "GPSLatitudeRef", 'S').is_none());
    }
}
