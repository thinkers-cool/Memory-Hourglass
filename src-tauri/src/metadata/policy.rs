use std::path::{Path, PathBuf};

use super::sidecar::{workspace_sidecar_path, xmp_sidecar_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataPolicy {
    InPlace,
    WorkspaceSidecar,
}

#[derive(Debug, Clone)]
pub struct MetadataContext {
    pub policy: MetadataPolicy,
    pub media_path: PathBuf,
    pub root_id: i64,
    pub rel_path: String,
    pub workspace_xmp_dir: PathBuf,
}

impl MetadataContext {
    pub fn in_place(media_path: PathBuf) -> Self {
        Self {
            policy: MetadataPolicy::InPlace,
            media_path,
            root_id: 0,
            rel_path: String::new(),
            workspace_xmp_dir: PathBuf::new(),
        }
    }

    pub fn workspace_sidecar(
        media_path: PathBuf,
        root_id: i64,
        rel_path: String,
        workspace_xmp_dir: PathBuf,
    ) -> Self {
        Self {
            policy: MetadataPolicy::WorkspaceSidecar,
            media_path,
            root_id,
            rel_path,
            workspace_xmp_dir,
        }
    }

    pub fn write_sidecar_path(&self) -> PathBuf {
        match self.policy {
            MetadataPolicy::InPlace => xmp_sidecar_path(&self.media_path),
            MetadataPolicy::WorkspaceSidecar => workspace_sidecar_path(
                &self.workspace_xmp_dir,
                self.root_id,
                &self.rel_path,
            ),
        }
    }

    pub fn colocated_sidecar_path(&self) -> PathBuf {
        xmp_sidecar_path(&self.media_path)
    }

    pub fn is_read_only_workspace(&self) -> bool {
        self.policy == MetadataPolicy::WorkspaceSidecar
    }
}

pub fn metadata_context_for_asset(
    read_only: bool,
    workspace_xmp_dir: &Path,
    root_id: i64,
    rel_path: &str,
    media_path: PathBuf,
) -> MetadataContext {
    if read_only {
        MetadataContext::workspace_sidecar(
            media_path,
            root_id,
            rel_path.to_string(),
            workspace_xmp_dir.to_path_buf(),
        )
    } else {
        MetadataContext::in_place(media_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_context_paths_and_read_only_flag() {
        let media = PathBuf::from("/photos/a.jpg");
        let in_place = MetadataContext::in_place(media.clone());
        assert!(!in_place.is_read_only_workspace());
        assert_eq!(in_place.write_sidecar_path(), xmp_sidecar_path(&media));
        assert_eq!(in_place.colocated_sidecar_path(), xmp_sidecar_path(&media));

        let xmp_dir = PathBuf::from("/xmp");
        let workspace = MetadataContext::workspace_sidecar(
            media.clone(),
            9,
            "a.jpg".into(),
            xmp_dir.clone(),
        );
        assert!(workspace.is_read_only_workspace());
        assert_eq!(
            workspace.write_sidecar_path(),
            workspace_sidecar_path(&xmp_dir, 9, "a.jpg")
        );

        let read_only = metadata_context_for_asset(true, &xmp_dir, 9, "a.jpg", media.clone());
        assert!(read_only.is_read_only_workspace());
        let writable = metadata_context_for_asset(false, &xmp_dir, 9, "a.jpg", media);
        assert!(!writable.is_read_only_workspace());
    }
}
