use crate::error::{AppError, Result};
use image::imageops::FilterType;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const THUMB_MAX_EDGE: u32 = 512;

pub struct ThumbService {
    dir: PathBuf,
}

impl ThumbService {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn thumb_path(&self, asset_id: i64) -> PathBuf {
        self.dir.join(format!("{}.webp", asset_id))
    }

    pub fn ensure_thumbnail(&self, asset_id: i64, source: &Path) -> Result<String> {
        let out = self.thumb_path(asset_id);
        if out.exists() {
            return Ok(out.file_name().unwrap().to_string_lossy().to_string());
        }

        let bytes = std::fs::read(source)?;
        self.ensure_thumbnail_from_bytes(asset_id, source, &bytes)
    }

    pub fn ensure_thumbnail_from_bytes(
        &self,
        asset_id: i64,
        source: &Path,
        bytes: &[u8],
    ) -> Result<String> {
        self.ensure_thumbnail_from_bytes_with_embedded(asset_id, source, bytes, None)
    }

    pub fn ensure_thumbnail_from_bytes_with_embedded(
        &self,
        asset_id: i64,
        source: &Path,
        bytes: &[u8],
        embedded_exif_thumb: Option<&[u8]>,
    ) -> Result<String> {
        if self.thumb_path(asset_id).exists() {
            return Ok(self
                .thumb_path(asset_id)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string());
        }
        if let Some(exif_thumb) = embedded_exif_thumb {
            if let Ok(thumb_key) = self.write_webp_from_bytes(asset_id, source, exif_thumb) {
                return Ok(thumb_key);
            }
        }
        if let Ok(thumb_key) = self.write_webp_from_bytes(asset_id, source, bytes) {
            return Ok(thumb_key);
        }
        if let Ok((preview_path, preview)) = load_embedded_preview(source) {
            return self.write_webp_from_bytes(asset_id, &preview_path, &preview);
        }
        Err(AppError::Metadata("thumbnail generation failed".into()))
    }

    fn write_webp_from_bytes(&self, asset_id: i64, source: &Path, bytes: &[u8]) -> Result<String> {
        let _ = image_format_from_path(source)?;
        let img = image::load_from_memory(bytes)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
        self.resize_and_save(asset_id, img)
    }

    fn resize_and_save(&self, asset_id: i64, img: image::DynamicImage) -> Result<String> {
        std::fs::create_dir_all(&self.dir)?;
        let thumb = img.resize(THUMB_MAX_EDGE, THUMB_MAX_EDGE, FilterType::Triangle);
        let out = self.thumb_path(asset_id);
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        thumb
            .write_to(&mut cursor, image::ImageFormat::WebP)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
        std::fs::write(&out, buffer)?;
        Ok(out.file_name().unwrap().to_string_lossy().to_string())
    }
}

fn image_format_from_path(path: &Path) -> Result<image::ImageFormat> {
    let ext = crate::path_util::os_extension(path).to_lowercase();
    if ext.is_empty() {
        return Err(AppError::Metadata("missing image extension".into()));
    }
    match ext.as_str() {
        "jpg" | "jpeg" => Ok(image::ImageFormat::Jpeg),
        "png" => Ok(image::ImageFormat::Png),
        "gif" => Ok(image::ImageFormat::Gif),
        "webp" => Ok(image::ImageFormat::WebP),
        "tif" | "tiff" => Ok(image::ImageFormat::Tiff),
        "bmp" => Ok(image::ImageFormat::Bmp),
        other => Err(AppError::Metadata(format!(
            "unsupported image extension: {}",
            other
        ))),
    }
}

fn embedded_preview_path(source: &Path) -> Result<PathBuf> {
    let stem = crate::path_util::os_file_stem(source);
    if stem.is_empty() {
        return Err(AppError::Metadata("no embedded preview".into()));
    }
    let parent = source
        .parent()
        .ok_or_else(|| AppError::Metadata("no embedded preview".into()))?;
    for ext in ["jpg", "jpeg", "JPG", "JPEG"] {
        let candidate = parent.join(format!("{}.{}", stem, ext));
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(AppError::Metadata("no embedded preview".into()))
}

fn load_embedded_preview(source: &Path) -> Result<(PathBuf, Vec<u8>)> {
    let path = embedded_preview_path(source)?;
    let bytes = std::fs::read(&path).map_err(AppError::from)?;
    Ok((path, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generates_webp_thumbnail() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("input.jpg");
        std::fs::write(&source, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();

        let thumb_dir = dir.path().join("thumbs");
        let svc = ThumbService::new(thumb_dir);
        let key = svc.ensure_thumbnail(1, &source).unwrap();
        assert!(key.ends_with(".webp"));
        assert!(svc.thumb_path(1).exists());
    }

    #[test]
    fn ensure_thumbnail_reuses_existing_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("cached.jpg");
        std::fs::write(&source, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let svc = ThumbService::new(thumb_dir);
        let first = svc.ensure_thumbnail(9, &source).unwrap();
        let second = svc.ensure_thumbnail(9, &source).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn generates_thumbnail_from_png_source() {
        use image::{ImageBuffer, Rgb};

        let dir = tempdir().unwrap();
        let source = dir.path().join("input.png");
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(4, 4, Rgb([120, 80, 40]));
        img.save(&source).unwrap();
        let svc = ThumbService::new(dir.path().join("thumbs"));
        let key = svc.ensure_thumbnail(2, &source).unwrap();
        assert!(key.ends_with(".webp"));
    }

    #[test]
    fn extract_embedded_preview_reads_sibling_jpeg() {
        let dir = tempdir().unwrap();
        let raw = dir.path().join("DSC011.cr2");
        let preview = dir.path().join("DSC011.jpg");
        std::fs::write(&raw, b"raw").unwrap();
        std::fs::write(&preview, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let (_, bytes) = load_embedded_preview(&raw).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn image_format_from_path_maps_extensions() {
        assert_eq!(
            image_format_from_path(Path::new("a.JPG")).unwrap(),
            image::ImageFormat::Jpeg
        );
        assert_eq!(
            image_format_from_path(Path::new("a.png")).unwrap(),
            image::ImageFormat::Png
        );
        assert_eq!(
            image_format_from_path(Path::new("a.gif")).unwrap(),
            image::ImageFormat::Gif
        );
        assert_eq!(
            image_format_from_path(Path::new("a.webp")).unwrap(),
            image::ImageFormat::WebP
        );
        assert_eq!(
            image_format_from_path(Path::new("a.tiff")).unwrap(),
            image::ImageFormat::Tiff
        );
        assert_eq!(
            image_format_from_path(Path::new("a.tif")).unwrap(),
            image::ImageFormat::Tiff
        );
        assert_eq!(
            image_format_from_path(Path::new("a.bmp")).unwrap(),
            image::ImageFormat::Bmp
        );
        assert!(image_format_from_path(Path::new("noext")).is_err());
        assert!(image_format_from_path(Path::new("file.xyz")).is_err());
    }

    #[test]
    fn extract_embedded_preview_errors_without_sibling_jpeg() {
        let dir = tempdir().unwrap();
        let raw = dir.path().join("orphan.cr2");
        std::fs::write(&raw, b"raw").unwrap();
        let err = load_embedded_preview(&raw).unwrap_err();
        assert!(err.to_string().contains("no embedded preview"));
    }

    #[test]
    fn extract_embedded_preview_skips_non_matching_extensions() {
        let dir = tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        std::fs::write(&raw, b"raw").unwrap();
        std::fs::write(dir.path().join("photo.png"), b"png").unwrap();
        assert!(load_embedded_preview(&raw).is_err());
    }

    #[test]
    fn extract_embedded_preview_errors_for_relative_path_without_parent() {
        let err = load_embedded_preview(Path::new("orphan.cr2")).unwrap_err();
        assert!(err.to_string().contains("no embedded preview"));
    }

    #[test]
    fn resize_and_save_reports_create_dir_failure() {
        let dir = tempdir().unwrap();
        let blocker = dir.path().join("not-a-dir");
        std::fs::write(&blocker, b"x").unwrap();
        let svc = ThumbService::new(blocker.join("thumbs"));
        let img =
            image::load_from_memory(include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let err = svc.resize_and_save(1, img).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn ensure_thumbnail_skips_corrupt_embedded_preview() {
        let dir = tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        let preview = dir.path().join("photo.jpg");
        std::fs::write(&raw, b"raw").unwrap();
        std::fs::write(&preview, b"not-an-image").unwrap();
        let svc = ThumbService::new(dir.path().join("thumbs"));
        assert!(svc.ensure_thumbnail(6, &raw).is_err());
    }

    #[test]
    fn ensure_thumbnail_rejects_corrupt_source_bytes() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("bad.jpg");
        std::fs::write(&source, b"not-an-image").unwrap();
        let svc = ThumbService::new(dir.path().join("thumbs"));
        assert!(svc.ensure_thumbnail(4, &source).is_err());
    }

    #[test]
    fn ensure_thumbnail_uses_embedded_preview_for_raw_source() {
        let dir = tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        let preview = dir.path().join("photo.jpg");
        std::fs::write(&raw, b"raw").unwrap();
        std::fs::write(&preview, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let svc = ThumbService::new(dir.path().join("thumbs"));
        let key = svc.ensure_thumbnail(5, &raw).unwrap();
        assert!(key.ends_with(".webp"));
    }

    #[test]
    fn extract_embedded_preview_reads_uppercase_jpeg_sibling() {
        let dir = tempdir().unwrap();
        let raw = dir.path().join("photo.cr2");
        let preview = dir.path().join("photo.JPEG");
        std::fs::write(&raw, b"raw").unwrap();
        std::fs::write(&preview, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let (_, bytes) = load_embedded_preview(&raw).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn ensure_thumbnail_from_gif_source() {
        use image::{ImageBuffer, Rgb};

        let dir = tempdir().unwrap();
        let source = dir.path().join("anim.gif");
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(4, 4, Rgb([200, 100, 50]));
        img.save(&source).unwrap();
        let svc = ThumbService::new(dir.path().join("thumbs"));
        let key = svc.ensure_thumbnail(8, &source).unwrap();
        assert!(key.ends_with(".webp"));
    }

    #[cfg(unix)]
    #[test]
    fn resize_and_save_reports_write_failure() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let thumb_dir = dir.path().join("thumbs");
        std::fs::create_dir_all(&thumb_dir).unwrap();
        std::fs::set_permissions(&thumb_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        let svc = ThumbService::new(thumb_dir);
        let img =
            image::load_from_memory(include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let err = svc.resize_and_save(3, img).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn extract_embedded_preview_errors_without_file_stem() {
        let err = load_embedded_preview(Path::new(".")).unwrap_err();
        assert!(err.to_string().contains("no embedded preview"));
    }

    #[test]
    fn extract_embedded_preview_errors_without_parent_directory() {
        let err = load_embedded_preview(Path::new("x")).unwrap_err();
        assert!(err.to_string().contains("no embedded preview"));
    }

    #[test]
    fn write_webp_from_bytes_errors_on_invalid_image_data() {
        let dir = tempdir().unwrap();
        let svc = ThumbService::new(dir.path().join("thumbs"));
        assert!(svc
            .write_webp_from_bytes(7, &dir.path().join("bad.jpg"), b"not-an-image")
            .is_err());
    }
}
