pub const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "heic", "heif", "tif", "tiff", "arw", "cr2", "cr3", "nef",
    "dng", "orf", "raf", "rw2", "pef", "srw", "mp4", "mov", "m4v", "avi", "mkv",
];

pub fn is_media_file(path: &std::path::Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with('.') {
        return false;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    MEDIA_EXTENSIONS.contains(&ext.as_str())
}

pub fn asset_kind(ext: &str) -> &'static str {
    let ext = ext.to_lowercase();
    match ext.as_str() {
        "arw" | "cr2" | "cr3" | "nef" | "dng" | "orf" | "raf" | "rw2" | "pef" | "srw" => "raw",
        "mp4" | "mov" | "m4v" | "avi" | "mkv" => "video",
        _ => "image",
    }
}

pub fn should_ignore(path: &std::path::Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with('.') {
        return true;
    }
    if name.ends_with(".tmp") || name.ends_with(".download") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_arw_as_media() {
        assert!(is_media_file(Path::new("/a/foo.ARW")));
        assert_eq!(asset_kind("arw"), "raw");
    }

    #[test]
    fn detects_common_image_and_video_extensions() {
        assert!(is_media_file(Path::new("/a/photo.jpg")));
        assert!(is_media_file(Path::new("/a/clip.MP4")));
        assert!(!is_media_file(Path::new("/a/readme.txt")));
        assert!(!is_media_file(Path::new("/a/.hidden.jpg")));
    }

    #[test]
    fn classifies_asset_kinds() {
        assert_eq!(asset_kind("jpg"), "image");
        assert_eq!(asset_kind("CR3"), "raw");
        assert_eq!(asset_kind("mov"), "video");
    }

    #[test]
    fn ignores_hidden_files() {
        assert!(should_ignore(Path::new("/a/.DS_Store")));
        assert!(!should_ignore(Path::new("/a/photo.jpg")));
    }

    #[test]
    fn ignores_temp_and_download_files() {
        assert!(should_ignore(Path::new("/a/photo.tmp")));
        assert!(should_ignore(Path::new("/a/photo.download")));
        assert!(!should_ignore(Path::new("/a/photo.jpg")));
    }
}
