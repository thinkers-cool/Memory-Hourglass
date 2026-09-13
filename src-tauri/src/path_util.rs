use crate::error::{AppError, Result};
use std::path::{Component, Path, PathBuf};

pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn os_file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn os_file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn os_extension(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn normalize_rel_path(rel_path: &str) -> String {
    rel_path.replace('\\', "/")
}

pub fn join_path_rel(base: &Path, rel: &str) -> PathBuf {
    let mut result = base.to_path_buf();
    for component in normalize_rel_path(rel).split('/').filter(|part| !part.is_empty()) {
        result.push(component);
    }
    result
}

pub fn join_root_rel(root: &str, rel: &str) -> String {
    let root_trimmed = root.trim_end_matches(['/', '\\']);
    let rel_normalized = normalize_rel_path(rel);
    if rel_normalized.is_empty() {
        return root_trimmed.to_string();
    }
    if root.starts_with('/') && !root.starts_with("//") {
        format!("{}/{}", root_trimmed, rel_normalized)
    } else {
        path_to_string(&join_path_rel(Path::new(root), rel))
    }
}

#[cfg(windows)]
fn prepare_canonicalize(path: &Path) -> PathBuf {
    let path_str = path_to_string(path).replace('/', "\\");
    if path_str.starts_with(r"\\?\") {
        return path.to_path_buf();
    }
    if path_str.starts_with(r"\\") {
        let without_prefix = path_str.trim_start_matches('\\');
        return PathBuf::from(format!(r"\\?\UNC\{}", without_prefix));
    }
    if path.is_absolute() {
        return PathBuf::from(format!(r"\\?\{}", path_str));
    }
    path.to_path_buf()
}

#[cfg(windows)]
fn simplify_stored_path(path: &Path) -> PathBuf {
    let path_str = path_to_string(path);
    if let Some(rest) = path_str.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", rest));
    }
    if let Some(rest) = path_str.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

#[cfg(not(windows))]
fn prepare_canonicalize(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(not(windows))]
fn simplify_stored_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

pub fn canonicalize(path: &Path) -> Result<PathBuf> {
    let prepared = prepare_canonicalize(path);
    let canonical = std::fs::canonicalize(&prepared).map_err(|_| {
        AppError::Library(format!("path not found: {}", path.display()))
    })?;
    Ok(simplify_stored_path(&canonical))
}

pub fn path_for_storage(path: &Path) -> Result<String> {
    Ok(path_to_string(&canonicalize(path)?))
}

pub fn paths_equal(a: &str, b: &str) -> bool {
    match (
        canonicalize(Path::new(a)),
        canonicalize(Path::new(b)),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}

pub fn is_unc_path(path: &Path) -> bool {
    let path_str = path_to_string(path);
    path_str.starts_with(r"\\")
        || path_str.starts_with(r"\\?\UNC\")
        || path_str.starts_with(r"\\?\")
            && path_str.len() > 4
            && path_str.as_bytes().get(4) == Some(&b'\\')
}

pub fn is_network_storage_path(path: &Path) -> bool {
    is_unc_path(path) || is_mapped_network_drive(path)
}

#[cfg(windows)]
fn is_mapped_network_drive(path: &Path) -> bool {
    const DRIVE_REMOTE: u32 = 4;
    let path_str = path_to_string(path);
    let bytes = path_str.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' {
        return false;
    }
    let drive_root = format!("{}\\", &path_str[..2]);
    drive_type(&drive_root) == DRIVE_REMOTE
}

#[cfg(windows)]
fn drive_type(root: &str) -> u32 {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    extern "system" {
        fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
    }

    let wide: Vec<u16> = OsStr::new(root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe { GetDriveTypeW(wide.as_ptr()) }
}

#[cfg(not(windows))]
fn is_mapped_network_drive(_: &Path) -> bool {
    false
}

pub fn validate_rel_path(rel_path: &str) -> Result<()> {
    let normalized = normalize_rel_path(rel_path);
    if normalized.is_empty() {
        return Err(AppError::InvalidInput("invalid path".into()));
    }
    if normalized.contains("..") || normalized.starts_with('/') {
        return Err(AppError::InvalidInput("invalid path".into()));
    }
    for component in Path::new(&normalized).components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::InvalidInput("invalid path".into()));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn normalize_rel_path_converts_backslashes() {
        assert_eq!(normalize_rel_path("photos\\vacation"), "photos/vacation");
    }

    #[test]
    fn validate_rel_path_accepts_forward_slashes() {
        assert!(validate_rel_path("photos/vacation").is_ok());
        assert!(validate_rel_path("photos\\vacation").is_ok());
    }

    #[test]
    fn validate_rel_path_rejects_traversal_and_absolute() {
        for rel in ["../secret", "/abs", ""] {
            assert!(validate_rel_path(rel).is_err());
        }
    }

    #[test]
    fn join_root_rel_uses_platform_separator() {
        let joined = join_root_rel(r"C:\photos", "vacation/DSC.jpg");
        assert!(joined.contains("vacation"));
        assert!(joined.contains("DSC.jpg"));
    }

    #[test]
    fn canonicalize_roundtrip_for_unicode_directory() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("测试");
        std::fs::create_dir_all(&nested).unwrap();
        let canonical = canonicalize(&nested).unwrap();
        assert!(canonical.is_dir());
        assert!(path_to_string(&canonical).contains("测试"));
    }

    #[test]
    fn paths_equal_treats_same_unicode_directory_as_equal() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("相册");
        std::fs::create_dir_all(&nested).unwrap();
        let stored = path_for_storage(&nested).unwrap();
        let via_string = path_to_string(&nested);
        assert!(paths_equal(&stored, &via_string));
    }

    #[test]
    fn os_file_name_preserves_unicode() {
        let path = Path::new(r"C:\photos\测试.jpg");
        assert_eq!(os_file_name(path), "测试.jpg");
    }

    #[cfg(windows)]
    #[test]
    fn is_unc_path_detects_extended_unc_prefix() {
        assert!(is_unc_path(Path::new(r"\\server\share")));
        assert!(is_unc_path(Path::new(r"\\?\UNC\server\share")));
    }
}
