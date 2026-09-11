use crate::catalog::models::{Asset, AssetMeta, AssetScanState, SourceRoot};
use crate::error::{AppError, Result};
use sqlx::SqlitePool;

pub struct SourceRootRepo {
    pool: SqlitePool,
}

struct InsertRootRequest<'a> {
    path: &'a str,
    kind: &'a str,
    scan_policy: &'a str,
    poll_secs: Option<i64>,
    smb_host: Option<&'a str>,
    smb_share: Option<&'a str>,
    smb_username: Option<&'a str>,
    smb_mounted: bool,
}

pub struct UpsertAssetInput<'a> {
    pub root_id: i64,
    pub rel_path: &'a str,
    pub file_name: &'a str,
    pub ext: &'a str,
    pub kind: &'a str,
    pub size: i64,
    pub mtime_ns: i64,
    pub sync_state: &'a str,
}

impl SourceRootRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_root(
        &self,
        path: &str,
        kind: &str,
        scan_policy: &str,
        poll_secs: Option<i64>,
    ) -> Result<SourceRoot> {
        self.insert_root_with_smb(InsertRootRequest {
            path,
            kind,
            scan_policy,
            poll_secs,
            smb_host: None,
            smb_share: None,
            smb_username: None,
            smb_mounted: false,
        })
        .await
    }

    pub async fn insert_smb_mount_root(
        &self,
        path: &str,
        poll_secs: Option<i64>,
        host: &str,
        share: &str,
        username: &str,
    ) -> Result<SourceRoot> {
        self.insert_root_with_smb(InsertRootRequest {
            path,
            kind: "smb",
            scan_policy: "poll",
            poll_secs,
            smb_host: Some(host),
            smb_share: Some(share),
            smb_username: Some(username),
            smb_mounted: true,
        })
        .await
    }

    async fn insert_root_with_smb(&self, request: InsertRootRequest<'_>) -> Result<SourceRoot> {
        let InsertRootRequest {
            path,
            kind,
            scan_policy,
            poll_secs,
            smb_host,
            smb_share,
            smb_username,
            smb_mounted,
        } = request;
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO source_root (path, kind, scan_policy, poll_secs, status, smb_host, smb_share, smb_username, smb_mounted)
            VALUES (?, ?, ?, ?, 'idle', ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(path)
        .bind(kind)
        .bind(scan_policy)
        .bind(poll_secs)
        .bind(smb_host)
        .bind(smb_share)
        .bind(smb_username)
        .bind(if smb_mounted { 1 } else { 0 })
        .fetch_one(&self.pool)
        .await?;

        self.get_root(id).await
    }

    pub async fn get_root(&self, id: i64) -> Result<SourceRoot> {
        sqlx::query_as::<_, SourceRoot>("SELECT * FROM source_root WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("root {}", id)))
    }

    pub async fn list_roots(&self) -> Result<Vec<SourceRoot>> {
        Ok(
            sqlx::query_as::<_, SourceRoot>("SELECT * FROM source_root ORDER BY id")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn remove_root(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM source_root WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("root {}", id)));
        }
        Ok(())
    }

    pub async fn set_status(&self, id: i64, status: &str) -> Result<()> {
        sqlx::query("UPDATE source_root SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn touch_scan(&self, id: i64, at: i64) -> Result<()> {
        sqlx::query(
            "UPDATE source_root SET last_scan_at = ?, status = 'idle' WHERE id = ?",
        )
        .bind(at)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn relink_path(&self, id: i64, path: &str) -> Result<SourceRoot> {
        let r = sqlx::query("UPDATE source_root SET path = ? WHERE id = ?")
            .bind(path)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("root {}", id)));
        }
        self.get_root(id).await
    }
}

pub struct AssetRepo {
    pool: SqlitePool,
}

impl AssetRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_asset(&self, input: UpsertAssetInput<'_>) -> Result<Asset> {
        let UpsertAssetInput {
            root_id,
            rel_path,
            file_name,
            ext,
            kind,
            size,
            mtime_ns,
            sync_state,
        } = input;
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state, deleted_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
            ON CONFLICT(root_id, rel_path) DO UPDATE SET
                file_name = excluded.file_name,
                ext = excluded.ext,
                kind = excluded.kind,
                size = excluded.size,
                mtime_ns = excluded.mtime_ns,
                sync_state = excluded.sync_state
            WHERE deleted_at IS NULL
            RETURNING id
            "#,
        )
        .bind(root_id)
        .bind(rel_path)
        .bind(file_name)
        .bind(ext)
        .bind(kind)
        .bind(size)
        .bind(mtime_ns)
        .bind(sync_state)
        .fetch_one(&self.pool)
        .await?;

        self.get_asset(id).await
    }

    pub async fn upsert_assets_batch(
        &self,
        root_id: i64,
        inputs: &[UpsertAssetInput<'_>],
    ) -> Result<Vec<i64>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::with_capacity(inputs.len());
        for input in inputs {
            let id = sqlx::query_scalar::<_, i64>(
                r#"
                INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state, deleted_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
                ON CONFLICT(root_id, rel_path) DO UPDATE SET
                    file_name = excluded.file_name,
                    ext = excluded.ext,
                    kind = excluded.kind,
                    size = excluded.size,
                    mtime_ns = excluded.mtime_ns,
                    sync_state = excluded.sync_state
                WHERE deleted_at IS NULL
                RETURNING id
                "#,
            )
            .bind(root_id)
            .bind(input.rel_path)
            .bind(input.file_name)
            .bind(input.ext)
            .bind(input.kind)
            .bind(input.size)
            .bind(input.mtime_ns)
            .bind(input.sync_state)
            .fetch_one(&mut *tx)
            .await?;
            ids.push(id);
        }
        tx.commit().await?;
        Ok(ids)
    }

    pub async fn get_asset(&self, id: i64) -> Result<Asset> {
        sqlx::query_as::<_, Asset>(
            "SELECT * FROM asset WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("asset {}", id)))
    }

    pub async fn find_by_path(&self, root_id: i64, rel_path: &str) -> Result<Option<Asset>> {
        Ok(
            sqlx::query_as::<_, Asset>(
                "SELECT * FROM asset WHERE root_id = ? AND rel_path = ? AND deleted_at IS NULL",
            )
            .bind(root_id)
            .bind(rel_path)
            .fetch_optional(&self.pool)
            .await?,
        )
    }

    pub async fn list_paths_for_root(&self, root_id: i64) -> Result<Vec<(String, i64, i64)>> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT rel_path, mtime_ns, size FROM asset WHERE root_id = ? AND deleted_at IS NULL",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_scan_state_for_root(
        &self,
        root_id: i64,
    ) -> Result<Vec<AssetScanState>> {
        Ok(
            sqlx::query_as::<_, AssetScanState>(
                r#"
                SELECT
                    rel_path,
                    mtime_ns,
                    size,
                    indexed_mtime_ns,
                    thumb_key,
                    kind,
                    (
                        SELECT COUNT(*)
                        FROM asset_raw_tag rt
                        WHERE rt.asset_id = asset.id
                    ) AS raw_tag_count
                FROM asset
                WHERE root_id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(root_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    pub async fn list_deleted_paths_for_root(&self, root_id: i64) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT rel_path FROM asset WHERE root_id = ? AND deleted_at IS NOT NULL",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn find_by_path_including_deleted(
        &self,
        root_id: i64,
        rel_path: &str,
    ) -> Result<Option<Asset>> {
        Ok(
            sqlx::query_as::<_, Asset>(
                "SELECT * FROM asset WHERE root_id = ? AND rel_path = ?",
            )
            .bind(root_id)
            .bind(rel_path)
            .fetch_optional(&self.pool)
            .await?,
        )
    }

    pub async fn count_for_root(&self, root_id: i64) -> Result<(i64, i64)> {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM asset WHERE root_id = ? AND deleted_at IS NULL",
        )
        .bind(root_id)
        .fetch_one(&self.pool)
        .await?;
        let missing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM asset WHERE root_id = ? AND deleted_at IS NULL AND sync_state = 'missing'",
        )
        .bind(root_id)
        .fetch_one(&self.pool)
        .await?;
        Ok((total, missing))
    }

    pub async fn mark_missing(&self, root_id: i64, rel_paths: &[String]) -> Result<()> {
        if rel_paths.is_empty() {
            return Ok(());
        }
        let mut builder = sqlx::QueryBuilder::new(
            "UPDATE asset SET sync_state = 'missing' WHERE root_id = ",
        );
        builder.push_bind(root_id);
        builder.push(" AND rel_path IN (");
        for (index, rel) in rel_paths.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(rel);
        }
        builder.push(")");
        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_thumb_key(&self, asset_id: i64, thumb_key: &str) -> Result<()> {
        sqlx::query("UPDATE asset SET thumb_key = ? WHERE id = ?")
            .bind(thumb_key)
            .bind(asset_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_file_stats(
        &self,
        asset_id: i64,
        mtime_ns: i64,
        size: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE asset SET mtime_ns = ?, size = ? WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mtime_ns)
        .bind(size)
        .bind(asset_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_indexed(&self, asset_id: i64, mtime_ns: i64) -> Result<()> {
        sqlx::query(
            "UPDATE asset SET indexed_mtime_ns = ?, sync_state = 'ok' WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mtime_ns)
        .bind(asset_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_sync_state(&self, asset_id: i64, sync_state: &str) -> Result<()> {
        sqlx::query("UPDATE asset SET sync_state = ? WHERE id = ? AND deleted_at IS NULL")
            .bind(sync_state)
            .bind(asset_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn soft_delete(&self, ids: &[i64], at: i64) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut builder = sqlx::QueryBuilder::new(
            "UPDATE asset SET deleted_at = ",
        );
        builder.push_bind(at);
        builder.push(", sync_state = 'ok' WHERE deleted_at IS NULL AND id IN (");
        for (index, id) in ids.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(*id);
        }
        builder.push(")");
        let result = builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn get_deleted_at_map(&self, ids: &[i64]) -> Result<std::collections::HashMap<i64, i64>> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let mut builder = sqlx::QueryBuilder::new(
            "SELECT id, deleted_at FROM asset WHERE deleted_at IS NOT NULL AND id IN (",
        );
        for (index, id) in ids.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(*id);
        }
        builder.push(")");
        let rows = builder
            .build_query_as::<(i64, i64)>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().collect())
    }

    pub async fn restore_assets(&self, ids: &[i64]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut builder = sqlx::QueryBuilder::new(
            "UPDATE asset SET deleted_at = NULL WHERE deleted_at IS NOT NULL AND id IN (",
        );
        for (index, id) in ids.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(*id);
        }
        builder.push(")");
        let result = builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn purge_assets(
        &self,
        ids: &[i64],
        _workspace_paths: &crate::workspace::WorkspacePaths,
        read_only: bool,
    ) -> Result<u64> {
        if read_only {
            return Err(crate::error::AppError::InvalidInput(
                "cannot purge source files in a read-only workspace".into(),
            ));
        }
        let mut deleted = 0u64;
        for id in ids {
            let row = sqlx::query_as::<_, PurgeRow>(
                r#"
                SELECT a.rel_path, r.path as root_path
                FROM asset a
                JOIN source_root r ON r.id = a.root_id
                WHERE a.id = ?
                "#,
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

            purge_asset_row_files(row.as_ref())?;
            sqlx::query("DELETE FROM asset WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
            deleted += 1;
        }
        Ok(deleted)
    }
}

fn purge_asset_row_files(row: Option<&PurgeRow>) -> Result<()> {
    let Some(row) = row else { return Ok(()); };
    remove_purged_asset_files(&row.root_path, &row.rel_path)
}

fn remove_purged_asset_files(root_path: &str, rel_path: &str) -> Result<()> {
    let path = std::path::PathBuf::from(root_path).join(rel_path);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let sidecar = crate::metadata::xmp_sidecar_path(&path);
    if sidecar.exists() { std::fs::remove_file(&sidecar)?; }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct PurgeRow {
    rel_path: String,
    root_path: String,
}

pub struct AssetMetaRepo {
    pool: SqlitePool,
}

impl AssetMetaRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, meta: &AssetMeta) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO asset_meta (asset_id, capture_at, camera, lens, rating, latitude, longitude, keywords_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(asset_id) DO UPDATE SET
                capture_at = excluded.capture_at,
                camera = excluded.camera,
                lens = excluded.lens,
                rating = excluded.rating,
                latitude = excluded.latitude,
                longitude = excluded.longitude,
                keywords_json = excluded.keywords_json
            "#,
        )
        .bind(meta.asset_id)
        .bind(meta.capture_at)
        .bind(&meta.camera)
        .bind(&meta.lens)
        .bind(meta.rating)
        .bind(meta.latitude)
        .bind(meta.longitude)
        .bind(&meta.keywords_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, asset_id: i64) -> Result<Option<AssetMeta>> {
        Ok(
            sqlx::query_as::<_, AssetMeta>("SELECT * FROM asset_meta WHERE asset_id = ?")
                .bind(asset_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }
}

pub struct RawTagRepo {
    pool: SqlitePool,
}

impl RawTagRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_for_asset(
        &self,
        asset_id: i64,
    ) -> Result<Vec<crate::catalog::models::RawTag>> {
        Ok(sqlx::query_as::<_, crate::catalog::models::RawTag>(
            "SELECT name, value FROM asset_raw_tag WHERE asset_id = ? ORDER BY name, value",
        )
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn count_for_asset(&self, asset_id: i64) -> Result<i64> {
        Ok(sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM asset_raw_tag WHERE asset_id = ?",
        )
        .bind(asset_id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn replace_for_asset(
        &self,
        asset_id: i64,
        tags: &[crate::catalog::models::RawTag],
    ) -> Result<()> {
        sqlx::query("DELETE FROM asset_raw_tag WHERE asset_id = ?")
            .bind(asset_id)
            .execute(&self.pool)
            .await?;
        for tag in tags {
            sqlx::query("INSERT INTO asset_raw_tag (asset_id, name, value) VALUES (?, ?, ?)")
                .bind(asset_id)
                .bind(&tag.name)
                .bind(&tag.value)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TagRow {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Option<String>,
    pub asset_count: i64,
}

const TAG_ASSET_COUNT_SQL: &str = r#"
COALESCE((
  SELECT COUNT(DISTINCT at.asset_id)
  FROM asset_tag at
  JOIN asset a ON a.id = at.asset_id AND a.deleted_at IS NULL
  WHERE at.tag_id IN (
    WITH RECURSIVE descendants AS (
      SELECT id FROM tag WHERE id = t.id
      UNION ALL
      SELECT child.id FROM tag child INNER JOIN descendants d ON child.parent_id = d.id
    )
    SELECT id FROM descendants
  )
), 0) AS asset_count
"#;

pub struct TagRepo {
    pool: SqlitePool,
}

impl TagRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_ids_for_asset(&self, asset_id: i64) -> Result<Vec<i64>> {
        Ok(
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT t.id FROM tag t
                JOIN asset_tag at ON at.tag_id = t.id
                WHERE at.asset_id = ?
                ORDER BY t.name
                "#,
            )
            .bind(asset_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    pub async fn list_names_for_asset(&self, asset_id: i64) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT t.name FROM tag t
                JOIN asset_tag at ON at.tag_id = t.id
                WHERE at.asset_id = ?
                ORDER BY t.name
                "#,
            )
            .bind(asset_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    pub async fn list_all_names(&self) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar::<_, String>("SELECT name FROM tag ORDER BY name")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn list_asset_ids_for_tag(&self, tag_id: i64) -> Result<Vec<i64>> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT asset_id FROM asset_tag WHERE tag_id = ?")
                .bind(tag_id)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn list_tag_rows(&self) -> Result<Vec<TagRow>> {
        let sql = format!(
            r#"
            SELECT
              t.id,
              t.name,
              t.parent_id,
              t.color,
              {asset_count}
            FROM tag t
            ORDER BY t.name
            "#,
            asset_count = TAG_ASSET_COUNT_SQL,
        );
        Ok(sqlx::query_as::<_, TagRow>(&sql).fetch_all(&self.pool).await?)
    }

    pub async fn descendant_tag_ids(&self, tag_id: i64) -> Result<Vec<i64>> {
        Ok(
            sqlx::query_scalar::<_, i64>(
                r#"
                WITH RECURSIVE descendants AS (
                  SELECT id FROM tag WHERE id = ?
                  UNION ALL
                  SELECT t.id FROM tag t INNER JOIN descendants d ON t.parent_id = d.id
                )
                SELECT id FROM descendants
                "#,
            )
            .bind(tag_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    pub async fn expand_tag_ids_with_descendants(
        &self,
        tag_ids: &[i64],
    ) -> Result<Vec<i64>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut expanded = std::collections::HashSet::new();
        for tag_id in tag_ids {
            for id in self.descendant_tag_ids(*tag_id).await? {
                expanded.insert(id);
            }
        }
        let mut ids = expanded.into_iter().collect::<Vec<_>>();
        ids.sort_unstable();
        Ok(ids)
    }

    pub async fn append_tag_id_to_assets(&self, asset_ids: &[i64], tag_id: i64) -> Result<u64> {
        if asset_ids.is_empty() {
            return Ok(0);
        }
        let mut updated = 0u64;
        for asset_id in asset_ids {
            let result = sqlx::query(
                "INSERT OR IGNORE INTO asset_tag (asset_id, tag_id) VALUES (?, ?)",
            )
            .bind(asset_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;
            if result.rows_affected() > 0 {
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub async fn remove_tag_id_from_assets(&self, asset_ids: &[i64], tag_id: i64) -> Result<u64> {
        if asset_ids.is_empty() {
            return Ok(0);
        }
        let mut updated = 0u64;
        for asset_id in asset_ids {
            let result = sqlx::query("DELETE FROM asset_tag WHERE asset_id = ? AND tag_id = ?")
                .bind(asset_id)
                .bind(tag_id)
                .execute(&self.pool)
                .await?;
            if result.rows_affected() > 0 {
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub async fn create_tag(
        &self,
        name: &str,
        parent_id: Option<i64>,
        color: Option<&str>,
    ) -> Result<i64> {
        if let Some(parent_id) = parent_id {
            let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tag WHERE id = ?")
                .bind(parent_id)
                .fetch_one(&self.pool)
                .await?;
            if exists == 0 {
                return Err(AppError::NotFound(format!("parent tag {}", parent_id)));
            }
        }

        sqlx::query("INSERT OR IGNORE INTO tag (name, parent_id, color) VALUES (?, ?, ?)")
            .bind(name)
            .bind(parent_id)
            .bind(color)
            .execute(&self.pool)
            .await?;
        if color.is_some() {
            sqlx::query("UPDATE tag SET color = ? WHERE name = ?")
                .bind(color)
                .bind(name)
                .execute(&self.pool)
                .await?;
        }
        sqlx::query_scalar::<_, i64>("SELECT id FROM tag WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Catalog(e.to_string()))
    }

    pub async fn update_tag(
        &self,
        id: i64,
        name: &str,
        color: Option<&str>,
    ) -> Result<TagRow> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput("tag name is required".into()));
        }
        let result = sqlx::query("UPDATE tag SET name = ?, color = ? WHERE id = ?")
            .bind(trimmed)
            .bind(color)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("tag {}", id)));
        }
        self.fetch_tag_row(id).await
    }

    pub async fn delete_tag(&self, id: i64) -> Result<()> {
        let child_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tag WHERE parent_id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        if child_count > 0 {
            return Err(AppError::InvalidInput(
                "cannot delete tag with subtags".into(),
            ));
        }

        sqlx::query("DELETE FROM asset_tag WHERE tag_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let result = sqlx::query("DELETE FROM tag WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("tag {}", id)));
        }
        Ok(())
    }

    async fn fetch_tag_row(&self, id: i64) -> Result<TagRow> {
        let sql = format!(
            r#"
            SELECT
              t.id,
              t.name,
              t.parent_id,
              t.color,
              {asset_count}
            FROM tag t
            WHERE t.id = ?
            "#,
            asset_count = TAG_ASSET_COUNT_SQL,
        );
        sqlx::query_as::<_, TagRow>(&sql)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Catalog(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::catalog::models::AssetMeta;
    use tempfile::tempdir;

    async fn test_catalog() -> (Catalog, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("test.db")).await.unwrap();
        (catalog, dir)
    }

    #[tokio::test]
    async fn upsert_asset_creates_and_updates() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();

        let a1 = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 100,
                mtime_ns: 1,
                sync_state: "new",
            })
            .await
            .unwrap();
        assert_eq!(a1.sync_state, "new");

        let a2 = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 200,
                mtime_ns: 2,
                sync_state: "modified",
            })
            .await
            .unwrap();
        assert_eq!(a2.id, a1.id);
        assert_eq!(a2.size, 200);
        assert_eq!(a2.sync_state, "modified");
    }

    #[tokio::test]
    async fn soft_delete_hides_asset() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "b.jpg",
                file_name: "b.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        assets.soft_delete(&[asset.id], 99).await.unwrap();
        let err = assets.get_asset(asset.id).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn meta_and_tags_roundtrip() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let meta_repo = AssetMetaRepo::new(catalog.pool().clone());
        let tag_repo = TagRepo::new(catalog.pool().clone());

        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "c.jpg",
                file_name: "c.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: Some(1_700_000_000),
                camera: Some("ILCE-7C".into()),
                lens: None,
                rating: Some(4),
                latitude: None,
                longitude: None,
                keywords_json: Some(r#"["travel"]"#.into()),
            })
            .await
            .unwrap();

        let travel_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        let japan_id = tag_repo.create_tag("japan", None, None).await.unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], travel_id)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], japan_id)
            .await
            .unwrap();

        let meta = meta_repo.get(asset.id).await.unwrap().unwrap();
        assert_eq!(meta.rating, Some(4));
        let tag_ids = tag_repo.list_ids_for_asset(asset.id).await.unwrap();
        assert_eq!(tag_ids.len(), 2);
    }

    #[tokio::test]
    async fn delete_tag_removes_asset_links() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let tag_repo = TagRepo::new(catalog.pool().clone());

        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "d.jpg",
                file_name: "d.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        let tag_id = tag_repo.create_tag("travel", None, Some("#ff0000")).await.unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();

        tag_repo.delete_tag(tag_id).await.unwrap();

        let tag_ids = tag_repo.list_ids_for_asset(asset.id).await.unwrap();
        assert!(tag_ids.is_empty());
        let tags = tag_repo.list_tag_rows().await.unwrap();
        assert!(tags.iter().all(|tag| tag.id != tag_id));
    }

    #[tokio::test]
    async fn expand_tag_ids_includes_descendants() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pool().clone());

        let parent_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        let child_id = tag_repo
            .create_tag("japan", Some(parent_id), None)
            .await
            .unwrap();

        let expanded = tag_repo
            .expand_tag_ids_with_descendants(&[parent_id])
            .await
            .unwrap();
        assert_eq!(expanded, vec![parent_id, child_id]);
    }

    #[tokio::test]
    async fn parent_tag_asset_count_includes_descendants() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let tag_repo = TagRepo::new(catalog.pool().clone());

        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "e.jpg",
                file_name: "e.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        let parent_id = tag_repo.create_tag("places", None, None).await.unwrap();
        let child_id = tag_repo
            .create_tag("tokyo", Some(parent_id), None)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], child_id)
            .await
            .unwrap();

        let rows = tag_repo.list_tag_rows().await.unwrap();
        let parent = rows.iter().find(|row| row.id == parent_id).unwrap();
        let child = rows.iter().find(|row| row.id == child_id).unwrap();
        assert_eq!(parent.asset_count, 1);
        assert_eq!(child.asset_count, 1);
    }

    #[tokio::test]
    async fn source_root_not_found_errors() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let err = roots.get_root(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
        let err = roots.remove_root(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
        let err = roots.relink_path(999, "/tmp").await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn asset_not_found_and_empty_batch_helpers() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();

        let err = assets.get_asset(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
        assert!(assets.upsert_assets_batch(root.id, &[]).await.unwrap().is_empty());
        assert_eq!(assets.soft_delete(&[], 1).await.unwrap(), 0);
        assert!(assets.get_deleted_at_map(&[]).await.unwrap().is_empty());
        assert_eq!(assets.restore_assets(&[]).await.unwrap(), 0);
        assets.mark_missing(root.id, &[]).await.unwrap();
    }

    #[tokio::test]
    async fn mark_missing_updates_multiple_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        for name in ["one.jpg", "two.jpg"] {
            assets
                .upsert_asset(UpsertAssetInput {
                    root_id: root.id,
                    rel_path: name,
                    file_name: name,
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                })
                .await
                .unwrap();
        }
        assets
            .mark_missing(root.id, &["one.jpg".into(), "two.jpg".into()])
            .await
            .unwrap();
        let (_, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(missing, 2);
    }

    #[tokio::test]
    async fn soft_delete_and_restore_multiple_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let mut ids = Vec::new();
        for name in ["a.jpg", "b.jpg"] {
            let asset = assets
                .upsert_asset(UpsertAssetInput {
                    root_id: root.id,
                    rel_path: name,
                    file_name: name,
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                })
                .await
                .unwrap();
            ids.push(asset.id);
        }
        assert_eq!(assets.soft_delete(&ids, 42).await.unwrap(), 2);
        let deleted = assets.get_deleted_at_map(&ids).await.unwrap();
        assert_eq!(deleted.len(), 2);
        assert_eq!(assets.restore_assets(&ids).await.unwrap(), 2);
    }

    #[test]
    fn remove_purged_asset_files_skips_missing_media() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        remove_purged_asset_files(root.to_str().unwrap(), "missing.jpg").unwrap();
    }

    #[test]
    fn remove_purged_asset_files_deletes_media_and_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("purge.jpg");
        std::fs::write(&image, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let sidecar = crate::metadata::xmp_sidecar_path(&image);
        std::fs::write(&sidecar, b"xmp").unwrap();
        remove_purged_asset_files(root.to_str().unwrap(), "purge.jpg").unwrap();
        assert!(!image.exists());
        assert!(!sidecar.exists());
    }

    #[tokio::test]
    async fn insert_smb_mount_root_persists_smb_fields() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let smb_path = dir.path().join("smb-share");
        std::fs::create_dir_all(&smb_path).unwrap();
        let root = roots
            .insert_smb_mount_root(
                smb_path.to_str().unwrap(),
                Some(120),
                "nas",
                "photos",
                "user",
            )
            .await
            .unwrap();
        assert_eq!(root.kind, "smb");
        assert_eq!(root.smb_host.as_deref(), Some("nas"));
        assert_eq!(root.smb_share.as_deref(), Some("photos"));
        assert_eq!(root.smb_username.as_deref(), Some("user"));
        assert_eq!(root.smb_mounted, 1);
    }

    #[tokio::test]
    async fn purge_assets_rejects_read_only_workspace() {
        let (catalog, dir) = test_catalog().await;
        let assets = AssetRepo::new(catalog.pool().clone());
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let err = assets.purge_assets(&[1], &paths, true).await.unwrap_err();
        assert!(err.to_string().contains("read-only"));
    }

    #[tokio::test]
    async fn purge_assets_removes_file_and_sidecar() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("photos");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        let image_path = photos.join("purge.jpg");
        std::fs::write(&image_path, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let sidecar = crate::metadata::xmp_sidecar_path(&image_path);
        std::fs::write(&sidecar, b"xmp").unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "purge.jpg",
                file_name: "purge.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let root_record = roots.get_root(root.id).await.unwrap();
        let disk_path = std::path::PathBuf::from(&root_record.path).join("purge.jpg");
        assert!(disk_path.exists());
        assert!(sidecar.exists());
        assets.purge_assets(&[asset.id], &paths, false).await.unwrap();
        assert!(!disk_path.exists());
        assert!(!sidecar.exists());
        let err = assets.get_asset(asset.id).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn purge_assets_noops_missing_files_for_unknown_id() {
        let (catalog, dir) = test_catalog().await;
        let assets = AssetRepo::new(catalog.pool().clone());
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let deleted = assets.purge_assets(&[999_999], &paths, false).await.unwrap();
        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn purge_assets_removes_file_without_sidecar() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("purge-no-xmp");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        let image_path = photos.join("only.jpg");
        std::fs::write(&image_path, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "only.jpg",
                file_name: "only.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        assets.purge_assets(&[asset.id], &paths, false).await.unwrap();
        assert!(!image_path.exists());
    }

    #[tokio::test]
    async fn raw_tag_count_and_replace() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "tagged.jpg",
                file_name: "tagged.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assert_eq!(raw_tag_repo.count_for_asset(asset.id).await.unwrap(), 0);
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .unwrap();
        assert_eq!(raw_tag_repo.count_for_asset(asset.id).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn tag_repo_empty_inputs_and_validation_errors() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pool().clone());

        assert!(tag_repo.expand_tag_ids_with_descendants(&[]).await.unwrap().is_empty());
        assert_eq!(tag_repo.append_tag_id_to_assets(&[], 1).await.unwrap(), 0);
        assert_eq!(tag_repo.remove_tag_id_from_assets(&[], 1).await.unwrap(), 0);

        let err = tag_repo.create_tag("child", Some(999), None).await.unwrap_err();
        assert!(err.to_string().contains("not found"));

        let tag_id = tag_repo.create_tag("rename-me", None, None).await.unwrap();
        let err = tag_repo.update_tag(tag_id, "   ", None).await.unwrap_err();
        assert!(err.to_string().contains("required"));
        let err = tag_repo.update_tag(999, "missing", None).await.unwrap_err();
        assert!(err.to_string().contains("not found"));

        let parent_id = tag_repo.create_tag("parent", None, None).await.unwrap();
        let _child_id = tag_repo.create_tag("child-tag", Some(parent_id), None).await.unwrap();
        let err = tag_repo.delete_tag(parent_id).await.unwrap_err();
        assert!(err.to_string().contains("subtags"));

        let err = tag_repo.delete_tag(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn relink_path_updates_root_path() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/old", "local", "watch", None).await.unwrap();
        let updated = roots.relink_path(root.id, "/tmp/new").await.unwrap();
        assert_eq!(updated.path, "/tmp/new");
    }

    #[tokio::test]
    async fn remove_root_deletes_existing_root() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/remove", "local", "watch", None).await.unwrap();
        roots.remove_root(root.id).await.unwrap();
        assert!(roots.list_roots().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn set_status_and_touch_scan_update_root_fields() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/status", "local", "watch", None).await.unwrap();
        roots.set_status(root.id, "scanning").await.unwrap();
        roots.touch_scan(root.id, 12345).await.unwrap();
        let updated = roots.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "idle");
        assert_eq!(updated.last_scan_at, Some(12345));
    }

    #[tokio::test]
    async fn upsert_asset_errors_when_path_soft_deleted() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assets.soft_delete(&[asset.id], 1).await.unwrap();
        assert!(assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 2,
                mtime_ns: 2,
                sync_state: "new",
            })
            .await
            .is_err());
    }

    #[tokio::test]
    async fn find_by_path_and_deleted_lookup_behaviors() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "live.jpg",
                file_name: "live.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assert!(assets.find_by_path(root.id, "missing.jpg").await.unwrap().is_none());
        assets.soft_delete(&[asset.id], 9).await.unwrap();
        assert!(assets.find_by_path(root.id, "live.jpg").await.unwrap().is_none());
        let deleted = assets
            .find_by_path_including_deleted(root.id, "live.jpg")
            .await
            .unwrap()
            .unwrap();
        assert!(deleted.deleted_at.is_some());
    }

    #[tokio::test]
    async fn list_paths_and_deleted_paths_filter_soft_deleted() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let live = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "live.jpg",
                file_name: "live.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let gone = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assets.soft_delete(&[gone.id], 1).await.unwrap();
        let paths = assets.list_paths_for_root(root.id).await.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].0, "live.jpg");
        let deleted_paths = assets.list_deleted_paths_for_root(root.id).await.unwrap();
        assert_eq!(deleted_paths, vec!["gone.jpg".to_string()]);
        let _ = live;
    }

    #[tokio::test]
    async fn count_for_root_and_mark_missing_unknown_paths() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg",
                file_name: "one.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assets.mark_missing(root.id, &["ghost.jpg".into()]).await.unwrap();
        let (total, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(missing, 0);
    }

    #[tokio::test]
    async fn mutators_skip_soft_deleted_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 10,
                mtime_ns: 10,
                sync_state: "new",
            })
            .await
            .unwrap();
        assets.soft_delete(&[asset.id], 1).await.unwrap();
        assets.update_file_stats(asset.id, 20, 20).await.unwrap();
        assets.mark_indexed(asset.id, 20).await.unwrap();
        assets.set_sync_state(asset.id, "missing").await.unwrap();
        let row = assets
            .find_by_path_including_deleted(root.id, "gone.jpg")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.size, 10);
        assert_eq!(row.sync_state, "ok");
        assert!(row.indexed_mtime_ns.is_none());
    }

    #[tokio::test]
    async fn set_thumb_key_and_batch_upsert_roundtrip() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let ids = assets
            .upsert_assets_batch(
                root.id,
                &[
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "a.jpg",
                        file_name: "a.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 1,
                        mtime_ns: 1,
                        sync_state: "new",
                    },
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "b.jpg",
                        file_name: "b.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 1,
                        mtime_ns: 1,
                        sync_state: "new",
                    },
                ],
            )
            .await
            .unwrap();
        assert_eq!(ids.len(), 2);
        assets.set_thumb_key(ids[0], "1.webp").await.unwrap();
        let asset = assets.get_asset(ids[0]).await.unwrap();
        assert_eq!(asset.thumb_key.as_deref(), Some("1.webp"));
    }

    #[tokio::test]
    async fn soft_delete_restore_and_purge_empty_helpers() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "x.jpg",
                file_name: "x.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assert_eq!(assets.soft_delete(&[asset.id], 1).await.unwrap(), 1);
        assert_eq!(assets.soft_delete(&[asset.id], 2).await.unwrap(), 0);
        assert_eq!(assets.restore_assets(&[asset.id]).await.unwrap(), 1);
        assert_eq!(assets.restore_assets(&[asset.id]).await.unwrap(), 0);
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        assert_eq!(assets.purge_assets(&[], &paths, false).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn asset_meta_get_and_upsert_update() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let meta_repo = AssetMetaRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "meta.jpg",
                file_name: "meta.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assert!(meta_repo.get(asset.id).await.unwrap().is_none());
        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(3),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(5),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        assert_eq!(meta_repo.get(asset.id).await.unwrap().unwrap().rating, Some(5));
    }

    #[tokio::test]
    async fn raw_tag_replace_clears_tags() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "raw.jpg",
                file_name: "raw.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .unwrap();
        raw_tag_repo.replace_for_asset(asset.id, &[]).await.unwrap();
        assert_eq!(raw_tag_repo.count_for_asset(asset.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn list_scan_state_for_root_includes_raw_tag_count() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "scan.jpg",
                file_name: "scan.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assets.set_thumb_key(asset.id, "thumb.webp").await.unwrap();
        assets.mark_indexed(asset.id, 1).await.unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .unwrap();
        let states = assets.list_scan_state_for_root(root.id).await.unwrap();
        let state = states.iter().find(|row| row.rel_path == "scan.jpg").unwrap();
        assert_eq!(state.raw_tag_count, 1);
        assert_eq!(state.thumb_key.as_deref(), Some("thumb.webp"));
    }

    #[tokio::test]
    async fn raw_tag_list_for_asset_returns_replaced_tags() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "listed.jpg",
                file_name: "listed.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Model".into(),
                    value: "Alpha".into(),
                }],
            )
            .await
            .unwrap();
        let tags = raw_tag_repo.list_for_asset(asset.id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Model");
    }

    #[tokio::test]
    async fn tag_listings_remove_update_and_duplicate_append() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "tagged.jpg",
                file_name: "tagged.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let parent_id = tag_repo.create_tag("places", None, None).await.unwrap();
        let child_id = tag_repo.create_tag("tokyo", Some(parent_id), None).await.unwrap();
        tag_repo.append_tag_id_to_assets(&[asset.id], child_id).await.unwrap();
        assert_eq!(tag_repo.append_tag_id_to_assets(&[asset.id], child_id).await.unwrap(), 0);
        let names = tag_repo.list_names_for_asset(asset.id).await.unwrap();
        assert_eq!(names, vec!["tokyo".to_string()]);
        let all_names = tag_repo.list_all_names().await.unwrap();
        assert!(all_names.contains(&"places".to_string()));
        let asset_ids = tag_repo.list_asset_ids_for_tag(child_id).await.unwrap();
        assert_eq!(asset_ids, vec![asset.id]);
        let descendants = tag_repo.descendant_tag_ids(parent_id).await.unwrap();
        assert_eq!(descendants, vec![parent_id, child_id]);
        let updated = tag_repo.update_tag(child_id, "osaka", Some("#00ff00")).await.unwrap();
        assert_eq!(updated.name, "osaka");
        assert_eq!(tag_repo.remove_tag_id_from_assets(&[asset.id], child_id).await.unwrap(), 1);
        assert!(tag_repo.list_names_for_asset(asset.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_tag_existing_name_reuses_id_and_sets_color() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let first_id = tag_repo.create_tag("shared", None, None).await.unwrap();
        let second_id = tag_repo
            .create_tag("shared", None, Some("#aabbcc"))
            .await
            .unwrap();
        assert_eq!(first_id, second_id);
        let rows = tag_repo.list_tag_rows().await.unwrap();
        let tag = rows.iter().find(|row| row.id == first_id).unwrap();
        assert_eq!(tag.color.as_deref(), Some("#aabbcc"));
    }

    #[tokio::test]
    async fn live_asset_mutators_and_find_by_path_success() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "live.jpg",
                file_name: "live.jpg",
                ext: "jpg",
                kind: "image",
                size: 10,
                mtime_ns: 10,
                sync_state: "new",
            })
            .await
            .unwrap();
        let found = assets.find_by_path(root.id, "live.jpg").await.unwrap().unwrap();
        assert_eq!(found.id, asset.id);
        assets.update_file_stats(asset.id, 20, 30).await.unwrap();
        assets.mark_indexed(asset.id, 20).await.unwrap();
        assets.set_sync_state(asset.id, "ok").await.unwrap();
        let updated = assets.get_asset(asset.id).await.unwrap();
        assert_eq!(updated.size, 30);
        assert_eq!(updated.sync_state, "ok");
        assert_eq!(updated.indexed_mtime_ns, Some(20));
    }

    #[tokio::test]
    async fn list_roots_returns_inserted_roots() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/list", "local", "watch", None).await.unwrap();
        let listed = roots.list_roots().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, root.id);
    }

    #[tokio::test]
    async fn remove_tag_id_from_assets_returns_zero_when_unassigned() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "plain.jpg",
                file_name: "plain.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tag_repo.create_tag("solo", None, None).await.unwrap();
        assert_eq!(tag_repo.remove_tag_id_from_assets(&[asset.id], tag_id).await.unwrap(), 0);
    }

    #[test]
    fn remove_purged_asset_files_errors_when_media_path_is_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(root.join("blocked.jpg")).unwrap();
        assert!(remove_purged_asset_files(root.to_str().unwrap(), "blocked.jpg").is_err());
    }

    #[test]
    fn remove_purged_asset_files_errors_when_sidecar_is_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("shot.jpg");
        std::fs::write(&image, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let sidecar = crate::metadata::xmp_sidecar_path(&image);
        std::fs::create_dir_all(&sidecar).unwrap();
        assert!(remove_purged_asset_files(root.to_str().unwrap(), "shot.jpg").is_err());
    }

    #[tokio::test]
    async fn source_root_repo_errors_after_pool_close() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        pool.close().await;
        assert!(roots.get_root(root.id).await.is_err());
        assert!(roots.list_roots().await.is_err());
        assert!(roots.remove_root(root.id).await.is_err());
        assert!(roots.set_status(root.id, "idle").await.is_err());
        assert!(roots.touch_scan(root.id, 1).await.is_err());
        assert!(roots.relink_path(root.id, "/tmp/new").await.is_err());
        assert!(roots
            .insert_root("/tmp/other", "local", "watch", None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn asset_repo_errors_after_pool_close() {
        let (catalog, dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        pool.close().await;
        assert!(assets.get_asset(asset.id).await.is_err());
        assert!(assets.find_by_path(root.id, "a.jpg").await.is_err());
        assert!(assets.list_paths_for_root(root.id).await.is_err());
        assert!(assets.list_scan_state_for_root(root.id).await.is_err());
        assert!(assets.list_deleted_paths_for_root(root.id).await.is_err());
        assert!(assets
            .find_by_path_including_deleted(root.id, "a.jpg")
            .await
            .is_err());
        assert!(assets.count_for_root(root.id).await.is_err());
        assert!(assets.mark_missing(root.id, &["a.jpg".into()]).await.is_err());
        assert!(assets.set_thumb_key(asset.id, "t").await.is_err());
        assert!(assets.update_file_stats(asset.id, 1, 1).await.is_err());
        assert!(assets.mark_indexed(asset.id, 1).await.is_err());
        assert!(assets.set_sync_state(asset.id, "ok").await.is_err());
        assert!(assets.soft_delete(&[asset.id], 1).await.is_err());
        assert!(assets.get_deleted_at_map(&[asset.id]).await.is_err());
        assert!(assets.restore_assets(&[asset.id]).await.is_err());
        assert!(assets.purge_assets(&[asset.id], &paths, false).await.is_err());
        assert!(assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "b.jpg",
                file_name: "b.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .is_err());
        assert!(assets
            .upsert_assets_batch(
                root.id,
                &[UpsertAssetInput {
                    root_id: root.id,
                    rel_path: "c.jpg",
                    file_name: "c.jpg",
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                }],
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn meta_and_raw_tag_repos_error_after_pool_close() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let meta_repo = AssetMetaRepo::new(pool.clone());
        let raw_tag_repo = RawTagRepo::new(pool.clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        pool.close().await;
        assert!(meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(1),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .is_err());
        assert!(meta_repo.get(asset.id).await.is_err());
        assert!(raw_tag_repo.list_for_asset(asset.id).await.is_err());
        assert!(raw_tag_repo.count_for_asset(asset.id).await.is_err());
        assert!(raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn upsert_assets_batch_errors_when_path_soft_deleted() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assets.soft_delete(&[asset.id], 1).await.unwrap();
        assert!(assets
            .upsert_assets_batch(
                root.id,
                &[UpsertAssetInput {
                    root_id: root.id,
                    rel_path: "gone.jpg",
                    file_name: "gone.jpg",
                    ext: "jpg",
                    kind: "image",
                    size: 2,
                    mtime_ns: 2,
                    sync_state: "new",
                }],
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn update_tag_fetch_errors_when_asset_table_is_dropped() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let tag_id = tag_repo.create_tag("rename", None, None).await.unwrap();
        sqlx::query("DROP TABLE asset")
            .execute(catalog.pool())
            .await
            .unwrap();
        let err = tag_repo.update_tag(tag_id, "renamed", None).await.unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn tag_mutations_error_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let tag_repo = TagRepo::new(pool.clone());
        let parent_id = tag_repo.create_tag("parent", None, None).await.unwrap();
        let tag_id = tag_repo.create_tag("leaf", None, None).await.unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(tag_repo
            .create_tag("blocked", Some(parent_id), Some("#fff"))
            .await
            .is_err());
        assert!(tag_repo.delete_tag(tag_id).await.is_err());
        sqlx::query("ROLLBACK")
            .execute(&mut *locker)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn raw_tag_replace_errors_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let raw_tag_repo = RawTagRepo::new(pool.clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "raw.jpg",
                file_name: "raw.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .is_err());
        sqlx::query("ROLLBACK")
            .execute(&mut *locker)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn purge_assets_errors_under_exclusive_lock_after_file_removed() {
        let (catalog, dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("purge-lock");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        let image_path = photos.join("gone.jpg");
        std::fs::write(&image_path, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(assets.purge_assets(&[asset.id], &paths, false).await.is_err());
        assert!(!image_path.exists());
        sqlx::query("ROLLBACK")
            .execute(&mut *locker)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn upsert_assets_batch_errors_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(assets
            .upsert_assets_batch(
                root.id,
                &[UpsertAssetInput {
                    root_id: root.id,
                    rel_path: "lock.jpg",
                    file_name: "lock.jpg",
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                }],
            )
            .await
            .is_err());
        sqlx::query("ROLLBACK")
            .execute(&mut *locker)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn purge_assets_propagates_filesystem_errors() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("purge-fs");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        std::fs::create_dir_all(photos.join("blocked.jpg")).unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "blocked.jpg",
                file_name: "blocked.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        assert!(assets.purge_assets(&[asset.id], &paths, false).await.is_err());
    }

    #[tokio::test]
    async fn tag_repo_errors_after_pool_close() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let tag_repo = TagRepo::new(pool.clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        tag_repo.append_tag_id_to_assets(&[asset.id], tag_id).await.unwrap();
        pool.close().await;
        assert!(tag_repo.list_ids_for_asset(asset.id).await.is_err());
        assert!(tag_repo.list_names_for_asset(asset.id).await.is_err());
        assert!(tag_repo.list_all_names().await.is_err());
        assert!(tag_repo.list_asset_ids_for_tag(tag_id).await.is_err());
        assert!(tag_repo.list_tag_rows().await.is_err());
        assert!(tag_repo.descendant_tag_ids(tag_id).await.is_err());
        assert!(tag_repo.expand_tag_ids_with_descendants(&[tag_id]).await.is_err());
        assert!(tag_repo.append_tag_id_to_assets(&[asset.id], tag_id).await.is_err());
        assert!(tag_repo.remove_tag_id_from_assets(&[asset.id], tag_id).await.is_err());
        assert!(tag_repo.create_tag("new", None, None).await.is_err());
        assert!(tag_repo.update_tag(tag_id, "new", None).await.is_err());
        assert!(tag_repo.delete_tag(tag_id).await.is_err());
    }

    #[tokio::test]
    async fn create_tag_rejects_missing_parent() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let err = tag_repo.create_tag("child", Some(999_999), None).await.unwrap_err();
        assert!(err.to_string().contains("parent tag"));
    }

    #[tokio::test]
    async fn create_tag_updates_color_for_existing_name() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let first = tag_repo.create_tag("colorful", None, Some("#111111")).await.unwrap();
        let second = tag_repo.create_tag("colorful", None, Some("#222222")).await.unwrap();
        assert_eq!(first, second);
        let rows = tag_repo.list_tag_rows().await.unwrap();
        let row = rows.iter().find(|row| row.id == first).unwrap();
        assert_eq!(row.color.as_deref(), Some("#222222"));
    }

    #[tokio::test]
    async fn delete_tag_returns_not_found_for_missing_id() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let err = tag_repo.delete_tag(999_999).await.unwrap_err();
        assert!(err.to_string().contains("tag"));
    }

    #[tokio::test]
    async fn replace_raw_tags_inserts_multiple_values() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "raw.jpg",
                file_name: "raw.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[
                    crate::catalog::models::RawTag {
                        name: "Make".into(),
                        value: "Sony".into(),
                    },
                    crate::catalog::models::RawTag {
                        name: "Model".into(),
                        value: "ILCE-7C".into(),
                    },
                ],
            )
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM asset_raw_tag WHERE asset_id = ?",
        )
        .bind(asset.id)
        .fetch_one(catalog.pool())
        .await
        .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn count_for_root_includes_missing_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "missing.jpg",
                file_name: "missing.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "missing",
            })
            .await
            .unwrap();
        let (total, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(missing, 1);
        assert_eq!(asset.sync_state, "missing");
    }

    #[tokio::test]
    async fn create_tag_fetch_errors_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let tag_repo = TagRepo::new(pool.clone());
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(tag_repo.create_tag("locked", None, Some("#abc")).await.is_err());
        sqlx::query("ROLLBACK")
            .execute(&mut *locker)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn upsert_assets_batch_commits_multiple_rows() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots.insert_root("/tmp/p", "local", "watch", None).await.unwrap();
        let ids = assets
            .upsert_assets_batch(
                root.id,
                &[
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "one.jpg",
                        file_name: "one.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 1,
                        mtime_ns: 1,
                        sync_state: "new",
                    },
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "two.jpg",
                        file_name: "two.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 2,
                        mtime_ns: 2,
                        sync_state: "new",
                    },
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "three.jpg",
                        file_name: "three.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 3,
                        mtime_ns: 3,
                        sync_state: "new",
                    },
                ],
            )
            .await
            .unwrap();
        assert_eq!(ids.len(), 3);
        let (total, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(total, 3);
        assert_eq!(missing, 0);
    }
}
