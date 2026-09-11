use crate::catalog::models::ExportOptions;
use crate::error::{AppError, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    pub destination: String,
    pub copied: Vec<String>,
    pub failed: Vec<ExportFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFailure {
    pub source: String,
    pub error: String,
}

pub struct ExportService {
    pool: SqlitePool,
}

pub type ExportProgressFn = Arc<dyn Fn(u64, u64, Option<&str>, &str) + Send + Sync>;

impl ExportService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn export_assets(
        &self,
        asset_ids: &[i64],
        destination: &Path,
        options: &ExportOptions,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ExportManifest> {
        self.export_assets_with_progress(
            asset_ids,
            destination,
            options,
            cancel,
            Arc::new(|_, _, _, _| {}),
        )
        .await
    }

    pub async fn export_assets_with_progress(
        &self,
        asset_ids: &[i64],
        destination: &Path,
        options: &ExportOptions,
        cancel: Option<Arc<AtomicBool>>,
        on_progress: ExportProgressFn,
    ) -> Result<ExportManifest> {
        if asset_ids.is_empty() {
            return Err(AppError::InvalidInput("no assets selected".into()));
        }
        std::fs::create_dir_all(destination)?;

        let total = asset_ids.len() as u64;
        let mut work_items = Vec::new();
        let mut failed = Vec::new();
        let mut processed = 0u64;

        for id in asset_ids {
            if cancel
                .as_ref()
                .map(|c| c.load(Ordering::SeqCst))
                .unwrap_or(false)
            {
                break;
            }
            let sql = format!(
                r#"
                SELECT a.file_name, r.path as root_path, a.rel_path, {} as capture_at, m.camera
                FROM asset a
                JOIN source_root r ON r.id = a.root_id
                LEFT JOIN asset_meta m ON m.asset_id = a.id
                WHERE a.id = ? AND a.deleted_at IS NULL
                "#,
                crate::dates::CAPTURE_AT_SQL,
            );
            let row = sqlx::query_as::<_, ExportRow>(&sql)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

            match row {
                None => {
                    failed.push(ExportFailure {
                        source: id.to_string(),
                        error: "asset not found".into(),
                    });
                    processed += 1;
                    on_progress(processed, total, None, "failed");
                }
                Some(r) => {
                    let root = PathBuf::from(&r.root_path);
                    let src = match crate::library::resolve_path_under_root(&root, &r.rel_path) {
                        Ok(path) => path,
                        Err(error) => {
                            failed.push(ExportFailure {
                                source: format!("{}/{}", r.root_path, r.rel_path),
                                error: error.to_string(),
                            });
                            processed += 1;
                            on_progress(processed, total, Some(&r.file_name), "failed");
                            continue;
                        }
                    };
                    let out_name = match resolve_export_name(
                        options.rename_template.as_deref(),
                        &r.file_name,
                        r.capture_at,
                        r.camera.as_deref(),
                    ) {
                        Ok(name) => name,
                        Err(error) => {
                            failed.push(ExportFailure {
                                source: src.to_string_lossy().to_string(),
                                error,
                            });
                            processed += 1;
                            on_progress(processed, total, Some(&r.file_name), "failed");
                            continue;
                        }
                    };
                    let dst = if options.flat {
                        match crate::library::validate_file_name(&out_name) {
                            Ok(()) => destination.join(&out_name),
                            Err(error) => {
                                failed.push(ExportFailure {
                                    source: src.to_string_lossy().to_string(),
                                    error: error.to_string(),
                                });
                                processed += 1;
                                on_progress(processed, total, Some(&r.file_name), "failed");
                                continue;
                            }
                        }
                    } else {
                        match crate::library::resolve_path_under_root(destination, &r.rel_path) {
                            Ok(path) => path,
                            Err(error) => {
                                failed.push(ExportFailure {
                                    source: src.to_string_lossy().to_string(),
                                    error: error.to_string(),
                                });
                                processed += 1;
                                on_progress(processed, total, Some(&r.file_name), "failed");
                                continue;
                            }
                        }
                    };
                    ensure_export_parent(&dst)?;
                    work_items.push(ExportWorkItem {
                        file_name: r.file_name,
                        src,
                        dst,
                    });
                }
            }
        }

        let mut copied = Vec::new();

        if !work_items.is_empty() {
            let format = options.format.clone();
            let converting = format.is_some();
            let done_phase = if converting { "converted" } else { "copied" };
            let processed_counter = Arc::new(AtomicU64::new(processed));
            let on_progress = on_progress.clone();
            let cancel = cancel.clone();
            let parallel_results = tokio::task::spawn_blocking(move || {
                if std::env::var_os("MEMHG_TEST_EXPORT_PARALLEL_PANIC").is_some() {
                    std::env::remove_var("MEMHG_TEST_EXPORT_PARALLEL_PANIC");
                    panic!("export parallel panic");
                }
                work_items
                    .par_iter()
                    .filter_map(|work| {
                        parallel_export_item(
                            work,
                            format.as_deref(),
                            cancel.as_ref(),
                            done_phase,
                            &processed_counter,
                            total,
                            &on_progress,
                        )
                    })
                    .collect::<Vec<ExportWorkOutcome>>()
            })
            .await
            .map_err(|e| AppError::Export(e.to_string()))?;

            for outcome in parallel_results {
                match outcome {
                    ExportWorkOutcome::Copied(path) => copied.push(path),
                    ExportWorkOutcome::Failed(failure) => failed.push(failure),
                }
            }
        }

        let manifest = ExportManifest {
            destination: destination.to_string_lossy().to_string(),
            copied,
            failed,
        };

        let status = if cancel
            .as_ref()
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(false)
        {
            "cancelled"
        } else if manifest.failed.is_empty() {
            "completed"
        } else if manifest.copied.is_empty() {
            "failed"
        } else {
            "partial"
        };

        sqlx::query("INSERT INTO export_job (status, manifest_json, created_at) VALUES (?, ?, ?)")
            .bind(status)
            .bind(serde_json::to_string(&manifest).unwrap())
            .bind(chrono::Utc::now().timestamp())
            .execute(&self.pool)
            .await?;

        Ok(manifest)
    }

    pub async fn latest_job(&self) -> Result<Option<(i64, String, ExportManifest)>> {
        let row = sqlx::query_as::<_, JobRow>(
            "SELECT id, status, manifest_json, created_at FROM export_job ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let manifest: ExportManifest =
                serde_json::from_str(&r.manifest_json).unwrap_or(ExportManifest {
                    destination: String::new(),
                    copied: vec![],
                    failed: vec![],
                });
            (r.id, r.status, manifest)
        }))
    }

    pub async fn list_jobs(&self) -> Result<Vec<(i64, String, i64, ExportManifest)>> {
        let rows = sqlx::query_as::<_, JobRow>(
            "SELECT id, status, manifest_json, created_at FROM export_job ORDER BY id DESC LIMIT 50",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let manifest: ExportManifest =
                    serde_json::from_str(&r.manifest_json).unwrap_or(ExportManifest {
                        destination: String::new(),
                        copied: vec![],
                        failed: vec![],
                    });
                (r.id, r.status, r.created_at, manifest)
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct ExportRow {
    file_name: String,
    root_path: String,
    rel_path: String,
    capture_at: Option<i64>,
    camera: Option<String>,
}

struct ExportWorkItem {
    file_name: String,
    src: PathBuf,
    dst: PathBuf,
}

enum ExportWorkOutcome {
    Copied(String),
    Failed(ExportFailure),
}

fn ensure_export_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn parallel_export_item(
    work: &ExportWorkItem,
    format: Option<&str>,
    cancel: Option<&Arc<AtomicBool>>,
    done_phase: &str,
    processed_counter: &AtomicU64,
    total: u64,
    on_progress: &ExportProgressFn,
) -> Option<ExportWorkOutcome> {
    if cancel
        .map(|flag| flag.load(Ordering::SeqCst))
        .unwrap_or(false)
    {
        return None;
    }
    let outcome = export_one(work, format);
    let done = processed_counter.fetch_add(1, Ordering::SeqCst) + 1;
    let phase = match &outcome {
        ExportWorkOutcome::Copied(_) => done_phase,
        ExportWorkOutcome::Failed(_) => "failed",
    };
    on_progress(done, total, Some(&work.file_name), phase);
    Some(outcome)
}

fn export_one(work: &ExportWorkItem, format: Option<&str>) -> ExportWorkOutcome {
    let result = if let Some(fmt) = format {
        convert_and_write(&work.src, &work.dst, fmt)
    } else {
        std::fs::copy(&work.src, &work.dst)
            .map(|_| ())
            .map_err(AppError::from)
    };
    match result {
        Ok(_) => ExportWorkOutcome::Copied(work.dst.to_string_lossy().to_string()),
        Err(e) => ExportWorkOutcome::Failed(ExportFailure {
            source: work.src.to_string_lossy().to_string(),
            error: e.to_string(),
        }),
    }
}

#[derive(sqlx::FromRow)]
struct JobRow {
    id: i64,
    status: String,
    manifest_json: String,
    created_at: i64,
}

fn resolve_export_name(
    template: Option<&str>,
    file_name: &str,
    capture_at: Option<i64>,
    camera: Option<&str>,
) -> std::result::Result<String, String> {
    match template {
        None => Ok(file_name.to_string()),
        Some(t) if t.trim().is_empty() => Ok(file_name.to_string()),
        Some(t) => {
            let path = PathBuf::from(file_name);
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| "missing file stem".to_string())?;
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .ok_or_else(|| "missing extension".to_string())?;

            if t.contains("{date}") && capture_at.is_none() {
                return Err("missing capture date for {date}".into());
            }
            if t.contains("{camera}") && camera.is_none() {
                return Err("missing camera for {camera}".into());
            }

            let date = capture_at
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .map(|d| d.format("%Y%m%d").to_string())
                .unwrap_or_default();
            let camera_name = camera.unwrap_or_default();

            let name = t
                .replace("{name}", stem)
                .replace("{date}", &date)
                .replace("{ext}", ext)
                .replace("{camera}", camera_name);
            if name.contains('.') {
                Ok(name)
            } else {
                Ok(format!("{}.{}", name, ext))
            }
        }
    }
}

fn convert_and_write(src: &Path, dst: &Path, format: &str) -> std::result::Result<(), AppError> {
    let img = image::open(src).map_err(|e| AppError::Export(e.to_string()))?;
    match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => {
            let out = dst.with_extension("jpg");
            img.save_with_format(&out, image::ImageFormat::Jpeg)
                .map_err(|e| AppError::Export(e.to_string()))?;
        }
        "webp" => {
            let out = dst.with_extension("webp");
            img.save_with_format(&out, image::ImageFormat::WebP)
                .map_err(|e| AppError::Export(e.to_string()))?;
        }
        "png" => {
            let out = dst.with_extension("png");
            img.save_with_format(&out, image::ImageFormat::Png)
                .map_err(|e| AppError::Export(e.to_string()))?;
        }
        other => {
            return Err(AppError::InvalidInput(format!(
                "unsupported format: {}",
                other
            )))
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo};
    use crate::catalog::Catalog;
    use crate::scan::{ScanControl, ScanService};
    use std::sync::Mutex;
    use tempfile::tempdir;

    fn noop_export_progress(_done: u64, _total: u64, _file_name: Option<&str>, _phase: &str) {
        std::hint::black_box(());
    }

    #[test]
    fn resolve_export_name_template() {
        let name = resolve_export_name(
            Some("{date}_{name}"),
            "photo.jpg",
            Some(1_700_000_000),
            Some("ILCE-7C"),
        )
        .unwrap();
        assert!(name.contains("photo"));
        assert!(name.ends_with(".jpg"));
    }

    #[test]
    fn resolve_export_name_whitespace_template_uses_original() {
        assert_eq!(
            resolve_export_name(Some("   "), "photo.jpg", None, None).unwrap(),
            "photo.jpg"
        );
    }

    #[test]
    fn resolve_export_name_fails_without_extension() {
        let err = resolve_export_name(Some("{name}"), "noext", None, None).unwrap_err();
        assert!(err.contains("extension"));
    }

    #[test]
    fn resolve_export_name_fails_without_stem() {
        let err = resolve_export_name(Some("{name}"), ".", None, None).unwrap_err();
        assert!(err.contains("stem"));
    }

    #[test]
    fn resolve_export_name_without_template() {
        assert_eq!(
            resolve_export_name(None, "photo.jpg", None, None).unwrap(),
            "photo.jpg"
        );
    }

    #[test]
    fn resolve_export_name_adds_extension_when_missing() {
        let name =
            resolve_export_name(Some("{camera}_{name}"), "photo.jpg", None, Some("Canon")).unwrap();
        assert!(name.ends_with(".jpg"));
        assert!(name.contains("Canon"));
    }

    #[test]
    fn resolve_export_name_keeps_extension_in_template() {
        let name = resolve_export_name(Some("archive.{ext}"), "photo.jpg", None, None).unwrap();
        assert_eq!(name, "archive.jpg");
    }

    #[test]
    fn resolve_export_name_fails_without_required_capture_date() {
        let err = resolve_export_name(Some("{date}_{name}"), "photo.jpg", None, None).unwrap_err();
        assert!(err.contains("capture date"));
    }

    #[test]
    fn resolve_export_name_fails_without_required_camera() {
        let err =
            resolve_export_name(Some("{camera}_{name}"), "photo.jpg", None, None).unwrap_err();
        assert!(err.contains("camera"));
    }

    #[tokio::test]
    async fn export_copies_files_flat() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("export.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        ScanService::new(catalog.pool().clone(), thumb_dir)
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "export.jpg")
            .await
            .unwrap()
            .unwrap();

        let dest = dir.path().join("out");
        let export = ExportService::new(catalog.pool().clone());
        let manifest = export
            .export_assets(
                &[asset.id],
                &dest,
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap();

        assert_eq!(manifest.copied.len(), 1);
        assert!(dest.join("export.jpg").exists());
    }

    #[tokio::test]
    async fn export_all_failures_record_failed_job_status() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let dest = dir.path().join("all-fail-out");
        export
            .export_assets(
                &[999_999, 999_998],
                &dest,
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        let latest = export.latest_job().await.unwrap().unwrap();
        assert_eq!(latest.1, "failed");
        assert!(latest.2.copied.is_empty());
        assert_eq!(latest.2.failed.len(), 2);
    }

    #[tokio::test]
    async fn export_reports_empty_selection_and_missing_assets() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let dest = dir.path().join("out");
        assert!(export
            .export_assets(
                &[],
                &dest,
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .is_err());
        let manifest = export
            .export_assets(
                &[999_999],
                &dest,
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(manifest.failed.len(), 1);
    }

    #[tokio::test]
    async fn export_honors_cancel_flag_and_nested_paths() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(photos.join("nested")).unwrap();
        std::fs::write(
            photos.join("nested/export.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "nested/export.jpg")
            .await
            .unwrap()
            .unwrap();

        let dest = dir.path().join("nested-out");
        let export = ExportService::new(catalog.pool().clone());
        let manifest = export
            .export_assets(
                &[asset.id],
                &dest,
                &ExportOptions {
                    flat: false,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(manifest.copied.len(), 1);
        assert!(dest.join("nested/export.jpg").exists());

        let cancel = Arc::new(AtomicBool::new(true));
        let cancelled = export
            .export_assets_with_progress(
                &[asset.id],
                &dir.path().join("cancel-out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                Some(cancel),
                Arc::new(noop_export_progress),
            )
            .await
            .unwrap();
        assert!(cancelled.copied.is_empty());
        noop_export_progress(0, 1, None, "failed");
    }

    #[tokio::test]
    async fn export_rename_template_errors_become_failures() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("bad.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "bad.jpg")
            .await
            .unwrap()
            .unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let manifest = export
            .export_assets(
                &[asset.id],
                &dir.path().join("rename-out"),
                &ExportOptions {
                    flat: true,
                    rename_template: Some("{date}_{name}".into()),
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(manifest.failed.len(), 1);
    }

    #[test]
    fn convert_and_write_supports_common_formats() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.jpg");
        std::fs::write(&src, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        for (format, ext) in [
            ("jpeg", "jpg"),
            ("jpg", "jpg"),
            ("webp", "webp"),
            ("png", "png"),
        ] {
            let dst = dir.path().join(format!("out.{}", ext));
            convert_and_write(&src, &dst, format).unwrap();
            assert!(dst.with_extension(ext).is_file());
        }
    }

    #[test]
    fn convert_and_write_rejects_unsupported_format() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.jpg");
        std::fs::write(&src, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let err = convert_and_write(&src, &dir.path().join("out.tiff"), "tiff").unwrap_err();
        assert!(err.to_string().contains("unsupported format"));
    }

    #[test]
    fn export_one_reports_copy_failure() {
        let dir = tempdir().unwrap();
        let work = ExportWorkItem {
            file_name: "missing.jpg".into(),
            src: dir.path().join("missing.jpg"),
            dst: dir.path().join("out.jpg"),
        };
        assert!(matches!(
            export_one(&work, None),
            ExportWorkOutcome::Failed(_)
        ));
        assert!(matches!(
            export_one(&work, Some("jpeg")),
            ExportWorkOutcome::Failed(_)
        ));
        let src = dir.path().join("src.jpg");
        std::fs::write(&src, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let success = ExportWorkItem {
            file_name: "src.jpg".into(),
            src,
            dst: dir.path().join("copied.jpg"),
        };
        let ExportWorkOutcome::Copied(path) = export_one(&success, None) else {
            panic!("expected copied outcome");
        };
        assert!(Path::new(&path).is_file());
    }

    #[tokio::test]
    async fn export_converts_formats_and_lists_jobs() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("convert.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "convert.jpg")
            .await
            .unwrap()
            .unwrap();

        let export = ExportService::new(catalog.pool().clone());
        for format in ["webp", "png", "jpeg"] {
            let dest = dir.path().join(format!("out-{}", format));
            let manifest = export
                .export_assets(
                    &[asset.id],
                    &dest,
                    &ExportOptions {
                        flat: true,
                        rename_template: None,
                        format: Some(format.into()),
                    },
                    None,
                )
                .await
                .unwrap();
            assert_eq!(manifest.copied.len(), 1);
        }

        let latest = export.latest_job().await.unwrap();
        assert!(latest.is_some());
        let jobs = export.list_jobs().await.unwrap();
        assert!(!jobs.is_empty());
    }

    #[tokio::test]
    async fn export_parallel_cancel_and_partial_status() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(photos.join("nested/deep")).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("nested/deep/one.jpg"), jpeg).unwrap();
        std::fs::write(photos.join("nested/deep/two.jpg"), jpeg).unwrap();
        std::fs::write(photos.join("nested/deep/three.jpg"), jpeg).unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let mut ids = Vec::new();
        for name in ["one.jpg", "two.jpg", "three.jpg"] {
            let asset = assets
                .find_by_path(root.id, &format!("nested/deep/{}", name))
                .await
                .unwrap()
                .unwrap();
            ids.push(asset.id);
        }

        let export = ExportService::new(catalog.pool().clone());
        let cancel = Arc::new(AtomicBool::new(true));
        let cancelled = export
            .export_assets_with_progress(
                &ids,
                &dir.path().join("cancel-mid"),
                &ExportOptions {
                    flat: false,
                    rename_template: None,
                    format: None,
                },
                Some(cancel),
                Arc::new(noop_export_progress),
            )
            .await
            .unwrap();
        noop_export_progress(0, 1, None, "failed");
        assert!(cancelled.copied.is_empty());
        assert_eq!(cancelled.failed.len(), 0);

        let partial = export
            .export_assets(
                &[ids[0], 999_999],
                &dir.path().join("partial"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(partial.copied.len(), 1);
        assert_eq!(partial.failed.len(), 1);

        std::fs::write(photos.join("nested/deep/corrupt.jpg"), b"not-an-image").unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let corrupt = assets
            .find_by_path(root.id, "nested/deep/corrupt.jpg")
            .await
            .unwrap()
            .unwrap();
        let converted = export
            .export_assets(
                &[corrupt.id],
                &dir.path().join("convert-nested"),
                &ExportOptions {
                    flat: false,
                    rename_template: None,
                    format: Some("jpeg".into()),
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(converted.copied.len(), 0);
        assert_eq!(converted.failed.len(), 1);
    }

    #[tokio::test]
    async fn export_nested_layout_creates_destination_dirs() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(photos.join("nested/deep")).unwrap();
        std::fs::write(
            photos.join("nested/deep/photo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "nested/deep/photo.jpg")
            .await
            .unwrap()
            .unwrap();
        let destination = dir.path().join("nested-export");
        let manifest = ExportService::new(catalog.pool().clone())
            .export_assets(
                &[asset.id],
                &destination,
                &ExportOptions {
                    flat: false,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(manifest.copied.len(), 1);
        assert!(destination.join("nested/deep/photo.jpg").is_file());
    }

    #[tokio::test]
    async fn export_parallel_cancel_after_first_item_progress() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        for index in 0..3 {
            std::fs::write(photos.join(format!("bulk-{index}.jpg")), jpeg).unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let ids = sqlx::query_scalar::<_, i64>("SELECT id FROM asset WHERE root_id = ?")
            .bind(root.id)
            .fetch_all(catalog.pool())
            .await
            .unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_flag = cancel.clone();
        export
            .export_assets_with_progress(
                &ids,
                &dir.path().join("cancel-after-first"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: Some("webp".into()),
                },
                Some(cancel),
                Arc::new(move |done, _, _, _| {
                    if done == 1 {
                        cancel_flag.store(true, Ordering::SeqCst);
                    }
                }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn export_parallel_conversion_cancelled_before_workers() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        for index in 0..4 {
            std::fs::write(photos.join(format!("bulk-{index}.jpg")), jpeg).unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let ids = sqlx::query_scalar::<_, i64>("SELECT id FROM asset WHERE root_id = ?")
            .bind(root.id)
            .fetch_all(catalog.pool())
            .await
            .unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let cancel = Arc::new(AtomicBool::new(true));
        let manifest = export
            .export_assets_with_progress(
                &ids,
                &dir.path().join("bulk-out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: Some("webp".into()),
                },
                Some(cancel),
                Arc::new(noop_export_progress),
            )
            .await
            .unwrap();
        assert!(manifest.copied.is_empty());
        noop_export_progress(0, 1, None, "failed");
    }

    #[tokio::test]
    async fn latest_job_tolerates_corrupt_manifest_json() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        sqlx::query("INSERT INTO export_job (status, manifest_json, created_at) VALUES (?, ?, ?)")
            .bind("completed")
            .bind("{bad-json")
            .bind(1_i64)
            .execute(catalog.pool())
            .await
            .unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let latest = export.latest_job().await.unwrap().unwrap();
        assert_eq!(latest.1, "completed");
        assert!(latest.2.copied.is_empty());
        let jobs = export.list_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].3.copied.is_empty());
    }

    #[test]
    fn parallel_export_item_skips_when_cancelled() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.jpg");
        std::fs::write(&src, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let work = ExportWorkItem {
            file_name: "src.jpg".into(),
            src,
            dst: dir.path().join("out.jpg"),
        };
        let cancel = Arc::new(AtomicBool::new(true));
        let processed = AtomicU64::new(0);
        let progress: ExportProgressFn = Arc::new(noop_export_progress);
        assert!(parallel_export_item(
            &work,
            None,
            Some(&cancel),
            "copied",
            &processed,
            1,
            &progress,
        )
        .is_none());
        noop_export_progress(0, 1, Some("src.jpg"), "failed");
    }

    #[tokio::test]
    async fn export_latest_job_returns_none_when_no_jobs() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let export = ExportService::new(catalog.pool().clone());
        assert!(export.latest_job().await.unwrap().is_none());
        assert!(export.list_jobs().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn export_records_completed_job_status() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("done.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "done.jpg")
            .await
            .unwrap()
            .unwrap();
        let export = ExportService::new(catalog.pool().clone());
        export
            .export_assets(
                &[asset.id],
                &dir.path().join("out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        let latest = export.latest_job().await.unwrap().unwrap();
        assert_eq!(latest.1, "completed");
    }

    #[tokio::test]
    async fn export_rename_template_success_with_capture_date() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("named.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "named.jpg")
            .await
            .unwrap()
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&crate::catalog::models::AssetMeta {
                asset_id: asset.id,
                capture_at: Some(1_700_000_000),
                camera: None,
                lens: None,
                rating: None,
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        let dest = dir.path().join("renamed-out");
        let export = ExportService::new(catalog.pool().clone());
        let manifest = export
            .export_assets(
                &[asset.id],
                &dest,
                &ExportOptions {
                    flat: true,
                    rename_template: Some("{date}_{name}".into()),
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(manifest.copied.len(), 1);
        assert!(dest.read_dir().unwrap().next().is_some());
    }

    #[tokio::test]
    async fn export_progress_reports_copied_and_converted_phases() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("phase.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "phase.jpg")
            .await
            .unwrap()
            .unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let copied_phases = Arc::new(Mutex::new(Vec::new()));
        let copied_capture = copied_phases.clone();
        export
            .export_assets_with_progress(
                &[asset.id],
                &dir.path().join("copy-out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
                Arc::new(move |_, _, _file_name, phase| {
                    copied_capture.lock().unwrap().push(phase.to_string());
                }),
            )
            .await
            .unwrap();
        assert!(copied_phases.lock().unwrap().iter().any(|p| p == "copied"));
        let converted_phases = Arc::new(Mutex::new(Vec::new()));
        let converted_capture = converted_phases.clone();
        export
            .export_assets_with_progress(
                &[asset.id],
                &dir.path().join("convert-out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: Some("webp".into()),
                },
                None,
                Arc::new(move |_, _, _file_name, phase| {
                    converted_capture.lock().unwrap().push(phase.to_string());
                }),
            )
            .await
            .unwrap();
        assert!(converted_phases
            .lock()
            .unwrap()
            .iter()
            .any(|p| p == "converted"));
    }

    #[test]
    fn ensure_export_parent_noops_when_no_parent() {
        let dir = tempdir().unwrap();
        ensure_export_parent(&dir.path().join("flat.jpg")).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn ensure_export_parent_propagates_create_dir_failure() {
        let dir = tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let nested = blocker.join("nested/out.jpg");
        assert!(ensure_export_parent(&nested).is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn export_fails_when_destination_parent_unwritable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("blocked.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "blocked.jpg")
            .await
            .unwrap()
            .unwrap();
        let blocker = dir.path().join("dest-blocker");
        std::fs::write(&blocker, b"x").unwrap();
        std::fs::set_permissions(&blocker, std::fs::Permissions::from_mode(0o444)).unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let err = export
            .export_assets(
                &[asset.id],
                &blocker.join("out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap_err();
        std::fs::set_permissions(&blocker, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn ensure_export_parent_accepts_relative_path_without_parent() {
        ensure_export_parent(Path::new("solo.jpg")).unwrap();
        ensure_export_parent(Path::new("nested/out.jpg")).unwrap();
    }

    #[test]
    fn convert_and_write_propagates_save_errors() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.jpg");
        std::fs::write(&src, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        for format in ["jpeg", "webp", "png"] {
            let err = convert_and_write(&src, &blocker.join("out.jpg"), format).unwrap_err();
            assert!(!err.to_string().is_empty());
        }
    }

    #[tokio::test]
    async fn export_parallel_propagates_join_error() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("panic.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "panic.jpg")
            .await
            .unwrap()
            .unwrap();
        std::env::set_var("MEMHG_TEST_EXPORT_PARALLEL_PANIC", "1");
        let export = ExportService::new(catalog.pool().clone());
        let err = export
            .export_assets(
                &[asset.id],
                &dir.path().join("panic-out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .unwrap_err();
        std::env::remove_var("MEMHG_TEST_EXPORT_PARALLEL_PANIC");
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn export_propagates_db_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("db.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pool.clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(pool.clone())
            .find_by_path(root.id, "db.jpg")
            .await
            .unwrap()
            .unwrap();
        pool.close().await;
        let export = ExportService::new(pool);
        assert!(export
            .export_assets(
                &[asset.id],
                &dir.path().join("closed-out"),
                &ExportOptions {
                    flat: true,
                    rename_template: None,
                    format: None,
                },
                None,
            )
            .await
            .is_err());
        assert!(export.latest_job().await.is_err());
        assert!(export.list_jobs().await.is_err());
    }

    #[tokio::test]
    async fn export_nested_rename_hits_ensure_export_parent() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(photos.join("nested")).unwrap();
        std::fs::write(
            photos.join("nested/photo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "nested/photo.jpg")
            .await
            .unwrap()
            .unwrap();
        let export = ExportService::new(catalog.pool().clone());
        let manifest = export
            .export_assets(
                &[asset.id],
                &dir.path().join("nested-out"),
                &ExportOptions {
                    flat: false,
                    rename_template: Some("{name}".into()),
                    format: None,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(manifest.copied.len(), 1);
        noop_export_progress(1, 1, Some("photo.jpg"), "copied");
    }
}
