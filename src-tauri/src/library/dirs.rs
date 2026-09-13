use crate::error::{AppError, Result};
use crate::path_util::{canonicalize, os_file_name, path_to_string};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub use crate::path_util::validate_rel_path;

#[derive(Debug, Clone, Serialize)]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
}

pub fn list_child_directories(path: &Path) -> Result<Vec<FolderEntry>> {
    #[cfg(test)]
    if std::env::var_os("MEMHG_TEST_LIST_DIRS_PANIC").is_some() {
        panic!("test list dirs panic");
    }
    if !path.is_dir() {
        return Err(AppError::Library(format!(
            "path is not a directory: {}",
            path.display()
        )));
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path).map_err(AppError::from)? {
        let entry = entry.map_err(AppError::from)?;
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        let name = os_file_name(&entry_path);
        if name.is_empty() || name.starts_with('.') {
            continue;
        }
        entries.push(FolderEntry {
            name,
            path: path_to_string(&entry_path),
        });
    }

    entries.sort_by_key(|entry| entry.name.to_lowercase());
    Ok(entries)
}

pub fn resolve_share_subfolder(mount_path: &Path, sub_path: Option<&str>) -> Result<PathBuf> {
    let mount_path =
        canonicalize(mount_path).map_err(|_| AppError::Library("SMB mount is not available".into()))?;

    let library_path = match sub_path {
        None | Some("") => mount_path.clone(),
        Some(sub_path) => {
            validate_sub_path(sub_path)?;
            let joined = crate::path_util::join_path_rel(&mount_path, sub_path);
            let canonical = canonicalize(&joined)
                .map_err(|_| AppError::Library(format!("folder not found: {}", sub_path)))?;
            if !canonical.starts_with(&mount_path) {
                return Err(AppError::InvalidInput("invalid folder path".into()));
            }
            canonical
        }
    };

    if !library_path.is_dir() {
        return Err(AppError::Library(
            "selected folder is not a directory".into(),
        ));
    }

    Ok(library_path)
}

fn validate_sub_path(sub_path: &str) -> Result<()> {
    validate_rel_path(sub_path)
}

pub fn validate_file_name(file_name: &str) -> Result<()> {
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains("..")
    {
        return Err(AppError::InvalidInput("invalid file name".into()));
    }
    Ok(())
}

pub fn resolve_path_under_root(root: &Path, rel_path: &str) -> Result<PathBuf> {
    validate_rel_path(rel_path)?;
    let root_canonical = canonicalize(root)
        .map_err(|_| AppError::Library(format!("root path not available: {}", root.display())))?;

    let mut resolved = root_canonical.clone();
    for component in Path::new(rel_path).components() {
        match component {
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err(AppError::InvalidInput("invalid path".into()));
            }
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => resolved.push(part),
        }
    }

    if resolved.exists() {
        let canonical = canonicalize(&resolved).map_err(AppError::from)?;
        if !canonical.starts_with(&root_canonical) {
            return Err(AppError::InvalidInput("invalid path".into()));
        }
        return Ok(canonical);
    }

    if let Some(parent) = resolved.parent() {
        if parent.exists() {
            let canonical_parent = canonicalize(parent).map_err(AppError::from)?;
            if !canonical_parent.starts_with(&root_canonical) {
                return Err(AppError::InvalidInput("invalid path".into()));
            }
        }
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn lists_child_directories_sorted_case_insensitive() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("beta")).unwrap();
        std::fs::create_dir_all(dir.path().join("Alpha")).unwrap();
        let entries = list_child_directories(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Alpha");
        assert_eq!(entries[1].name, "beta");
    }

    #[test]
    fn lists_child_directories_only() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Photos")).unwrap();
        std::fs::create_dir_all(dir.path().join(".hidden")).unwrap();
        std::fs::write(dir.path().join("readme.txt"), "x").unwrap();

        let entries = list_child_directories(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Photos");
    }

    #[test]
    fn rejects_invalid_sub_path() {
        let dir = tempdir().unwrap();
        assert!(resolve_share_subfolder(dir.path(), Some("../etc/passwd")).is_err());
    }

    #[test]
    fn validate_rel_path_rejects_traversal_and_absolute() {
        for rel in ["../secret", "/abs", "\\win", ""] {
            assert!(validate_rel_path(rel).is_err());
        }
        assert!(validate_rel_path("photos/vacation").is_ok());
        assert!(validate_rel_path("photos\\vacation").is_ok());
    }

    #[test]
    fn validate_file_name_rejects_path_separators() {
        for name in ["../x.jpg", "a/b.jpg", "a\\b.jpg", ""] {
            assert!(validate_file_name(name).is_err());
        }
    }

    #[test]
    fn resolve_path_under_root_accepts_nested_file() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("photos/vacation");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("shot.jpg");
        std::fs::write(&file, b"x").unwrap();
        let resolved = resolve_path_under_root(dir.path(), "photos/vacation/shot.jpg").unwrap();
        assert!(resolved.ends_with("shot.jpg"));
    }

    #[test]
    fn resolve_path_under_root_rejects_traversal() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("photos")).unwrap();
        assert!(resolve_path_under_root(dir.path(), "../outside.jpg").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resolve_path_under_root_rejects_symlink_escape() {
        let dir = tempdir().unwrap();
        let mount = dir.path().join("root");
        let nested = mount.join("photos");
        std::fs::create_dir_all(&nested).unwrap();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let link = nested.join("escape");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(resolve_path_under_root(&mount, "photos/escape").is_err());
    }

    #[test]
    fn resolve_share_subfolder_accepts_nested_directory() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("photos/vacation");
        std::fs::create_dir_all(&nested).unwrap();
        let resolved = resolve_share_subfolder(dir.path(), Some("photos/vacation")).unwrap();
        assert!(resolved.ends_with("vacation"));
    }

    #[test]
    fn resolve_share_subfolder_rejects_non_directory() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();
        assert!(resolve_share_subfolder(&file, None).is_err());
    }

    #[test]
    fn list_child_directories_rejects_missing_path() {
        let dir = tempdir().unwrap();
        assert!(list_child_directories(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn resolve_share_subfolder_returns_mount_root_without_subpath() {
        let dir = tempdir().unwrap();
        let resolved = resolve_share_subfolder(dir.path(), None).unwrap();
        assert_eq!(resolved, canonicalize(dir.path()).unwrap());
        let empty = resolve_share_subfolder(dir.path(), Some("")).unwrap();
        assert_eq!(empty, canonicalize(dir.path()).unwrap());
    }

    #[test]
    fn resolve_share_subfolder_rejects_missing_nested_path() {
        let dir = tempdir().unwrap();
        let err = resolve_share_subfolder(dir.path(), Some("missing/sub")).unwrap_err();
        assert!(err.to_string().contains("folder not found"));
    }

    #[test]
    fn resolve_share_subfolder_rejects_unavailable_mount() {
        let missing =
            std::path::PathBuf::from(format!("/tmp/memhg-missing-mount-{}", std::process::id()));
        let err = resolve_share_subfolder(&missing, None).unwrap_err();
        assert!(err.to_string().contains("not available"));
    }

    #[test]
    fn validate_sub_path_rejects_absolute_paths() {
        let dir = tempdir().unwrap();
        for sub in ["/abs", "\\win"] {
            assert!(resolve_share_subfolder(dir.path(), Some(sub)).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn list_child_directories_propagates_read_dir_failure() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let locked = dir.path().join("locked");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let err = list_child_directories(&locked).unwrap_err();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!err.to_string().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn resolve_share_subfolder_rejects_escape_outside_mount() {
        let dir = tempdir().unwrap();
        let mount = dir.path().join("mount");
        let nested = mount.join("photos");
        std::fs::create_dir_all(&nested).unwrap();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let traversal = mount.join("photos/link");
        std::os::unix::fs::symlink(&outside, &traversal).unwrap();
        assert!(resolve_share_subfolder(&mount, Some("photos/link")).is_err());
    }
}
