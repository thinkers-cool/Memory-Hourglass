use crate::error::{AppError, Result};
use crate::workspace::{WorkspaceInfo, MAX_RECENT};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const REGISTRY_FILE: &str = "workspaces.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentWorkspace {
    pub path: String,
    pub name: String,
    pub last_opened: i64,
    #[serde(default)]
    pub valid: bool,
    #[serde(default)]
    pub root_count: u32,
    #[serde(default)]
    pub album_count: u32,
    #[serde(default)]
    pub tag_count: u32,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkspaceRegistryFile {
    pub last_opened: Option<String>,
    pub recent: Vec<RecentWorkspace>,
}

pub struct WorkspaceRegistry {
    file: WorkspaceRegistryFile,
}

impl WorkspaceRegistry {
    pub fn load(app_data_dir: &Path) -> Result<Self> {
        let path = app_data_dir.join(REGISTRY_FILE);
        if !path.is_file() {
            return Ok(Self {
                file: WorkspaceRegistryFile::default(),
            });
        }
        let raw = std::fs::read_to_string(&path)?;
        let file: WorkspaceRegistryFile = serde_json::from_str(&raw)
            .map_err(|e| AppError::Workspace(format!("invalid workspaces registry: {}", e)))?;
        Ok(Self { file })
    }

    pub fn save(&self, app_data_dir: &Path) -> Result<()> {
        let path = app_data_dir.join(REGISTRY_FILE);
        let json = serde_json::to_string_pretty(&self.file)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn list_recent(&self) -> Vec<RecentWorkspace> {
        self.file.recent.clone()
    }

    pub fn last_opened(&self) -> Option<String> {
        self.file.last_opened.clone()
    }

    pub fn touch_recent(&mut self, info: &WorkspaceInfo) {
        let now = now_secs();
        self.file.recent.retain(|entry| entry.path != info.path);
        self.file.recent.insert(
            0,
            RecentWorkspace {
                path: info.path.clone(),
                name: info.name.clone(),
                last_opened: now,
                valid: false,
                root_count: 0,
                album_count: 0,
                tag_count: 0,
                read_only: info.read_only,
            },
        );
        if self.file.recent.len() > MAX_RECENT {
            self.file.recent.truncate(MAX_RECENT);
        }
        self.file.last_opened = Some(info.path.clone());
    }

    pub fn remove_recent(&mut self, path: &str) {
        self.file.recent.retain(|entry| entry.path != path);
        if self.file.last_opened.as_deref() == Some(path) {
            self.file.last_opened = None;
        }
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn touch_recent_stores_read_only_flag() {
        let mut registry = WorkspaceRegistry {
            file: WorkspaceRegistryFile::default(),
        };
        registry.touch_recent(&WorkspaceInfo {
            path: "/ws/readonly".into(),
            name: "Readonly".into(),
            id: "id".into(),
            read_only: true,
        });
        assert!(registry.list_recent()[0].read_only);
    }

    #[test]
    fn touch_recent_orders_and_caps() {
        let mut registry = WorkspaceRegistry {
            file: WorkspaceRegistryFile::default(),
        };
        for index in 0..12 {
            registry.touch_recent(&WorkspaceInfo {
                path: format!("/ws/{}", index),
                name: format!("WS {}", index),
                id: format!("id-{}", index),
                read_only: false,
            });
        }
        assert_eq!(registry.list_recent().len(), MAX_RECENT);
        assert_eq!(registry.list_recent()[0].path, "/ws/11");
        assert_eq!(registry.last_opened().as_deref(), Some("/ws/11"));
    }

    #[test]
    fn remove_recent_keeps_last_opened_for_other_paths() {
        let mut registry = WorkspaceRegistry {
            file: WorkspaceRegistryFile::default(),
        };
        registry.touch_recent(&WorkspaceInfo {
            path: "/ws/a".into(),
            name: "A".into(),
            id: "id-a".into(),
            read_only: false,
        });
        registry.touch_recent(&WorkspaceInfo {
            path: "/ws/b".into(),
            name: "B".into(),
            id: "id-b".into(),
            read_only: false,
        });
        registry.remove_recent("/ws/a");
        assert_eq!(registry.list_recent().len(), 1);
        assert_eq!(registry.last_opened().as_deref(), Some("/ws/b"));
    }

    #[test]
    fn remove_recent_clears_last_opened() {
        let mut registry = WorkspaceRegistry {
            file: WorkspaceRegistryFile::default(),
        };
        let info = WorkspaceInfo {
            path: "/ws/a".into(),
            name: "A".into(),
            id: "id".into(),
            read_only: false,
        };
        registry.touch_recent(&info);
        registry.remove_recent("/ws/a");
        assert!(registry.list_recent().is_empty());
        assert!(registry.last_opened().is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let mut registry = WorkspaceRegistry {
            file: WorkspaceRegistryFile::default(),
        };
        registry.touch_recent(&WorkspaceInfo {
            path: "/tmp/ws".into(),
            name: "Demo".into(),
            id: "id".into(),
            read_only: false,
        });
        registry.save(dir.path()).unwrap();
        let loaded = WorkspaceRegistry::load(dir.path()).unwrap();
        assert_eq!(loaded.list_recent(), registry.list_recent());
    }

    #[test]
    fn load_returns_empty_registry_when_file_missing() {
        let dir = tempdir().unwrap();
        let registry = WorkspaceRegistry::load(dir.path()).unwrap();
        assert!(registry.list_recent().is_empty());
    }

    #[test]
    fn load_rejects_invalid_registry_json() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(REGISTRY_FILE), "{bad").unwrap();
        assert!(WorkspaceRegistry::load(dir.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn save_fails_when_registry_file_unwritable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        let mut registry = WorkspaceRegistry {
            file: WorkspaceRegistryFile::default(),
        };
        registry.touch_recent(&WorkspaceInfo {
            path: "/tmp/ws".into(),
            name: "Demo".into(),
            id: "id".into(),
            read_only: false,
        });
        registry.save(dir.path()).unwrap();
        let path = dir.path().join(REGISTRY_FILE);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
        assert!(registry.save(dir.path()).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn load_fails_when_registry_unreadable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let registry = dir.path().join(REGISTRY_FILE);
        std::fs::write(&registry, r#"{"recent":[],"last_opened":null}"#).unwrap();
        std::fs::set_permissions(&registry, std::fs::Permissions::from_mode(0o000)).unwrap();
        assert!(WorkspaceRegistry::load(dir.path()).is_err());
        std::fs::set_permissions(&registry, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}
