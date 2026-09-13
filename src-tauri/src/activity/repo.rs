use crate::activity::models::ActivityEntry;
use crate::activity::models::ActivityInput;
use crate::error::Result;
use crate::catalog::pools::CatalogPools;
use sqlx::SqlitePool;
use sqlx::Transaction;

pub struct ActivityRepo {
    pools: CatalogPools,
}

impl ActivityRepo {
    pub fn new(pools: CatalogPools) -> Self {
        Self { pools }
    }

    pub fn pool(&self) -> &SqlitePool {
        self.pools.write()
    }

    pub async fn begin(&self) -> Result<Transaction<'_, sqlx::Sqlite>> {
        Ok(self.pools.write().begin().await?)
    }

    pub async fn next_seq(&self, tx: &mut Transaction<'_, sqlx::Sqlite>) -> Result<i64> {
        let row: (Option<i64>,) = sqlx::query_as("SELECT MAX(seq) FROM activity_log")
            .fetch_one(&mut **tx)
            .await?;
        Ok(row.0.unwrap_or(0) + 1)
    }

    pub async fn append_in_tx(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        input: ActivityInput,
    ) -> Result<i64> {
        let seq = self.next_seq(tx).await?;
        let occurred_at = chrono::Utc::now().timestamp();
        let result = sqlx::query(
            r#"
            INSERT INTO activity_log (
                seq, occurred_at, event_type, actor, correlation_id,
                subject_type, subject_id, subject_key, summary,
                payload_json, revert_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(seq)
        .bind(occurred_at)
        .bind(&input.event_type)
        .bind(&input.actor)
        .bind(&input.correlation_id)
        .bind(&input.subject_type)
        .bind(input.subject_id)
        .bind(&input.subject_key)
        .bind(&input.summary)
        .bind(&input.payload_json)
        .bind(&input.revert_json)
        .execute(&mut **tx)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get(&self, id: i64) -> Result<ActivityEntry> {
        let row = sqlx::query_as::<_, ActivityRow>(
            r#"
            SELECT id, seq, occurred_at, event_type, actor, correlation_id,
                   subject_type, subject_id, subject_key, summary,
                   payload_json, revert_json, undone_at
            FROM activity_log WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pools.read())
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound(format!("activity {} not found", id)))?;

        Ok(row.into_entry())
    }

    pub async fn list_for_asset(
        &self,
        asset_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ActivityEntry>> {
        let rows = sqlx::query_as::<_, ActivityRow>(
            r#"
            SELECT id, seq, occurred_at, event_type, actor, correlation_id,
                   subject_type, subject_id, subject_key, summary,
                   payload_json, revert_json, undone_at
            FROM activity_log
            WHERE subject_type = 'asset' AND subject_id = ?
            ORDER BY occurred_at DESC, id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(asset_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pools.read())
        .await?;

        Ok(rows.into_iter().map(|r| r.into_entry()).collect())
    }

    pub async fn mark_undone(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        id: i64,
        undone_by_id: i64,
    ) -> Result<()> {
        let at = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE activity_log SET undone_at = ?, undone_by_id = ? WHERE id = ? AND undone_at IS NULL",
        )
        .bind(at)
        .bind(undone_by_id)
        .bind(id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::models::actor;
    use crate::activity::models::event_type;
    use crate::activity::models::ActivityInput;
    use crate::catalog::Catalog;
    use tempfile::tempdir;

    #[tokio::test]
    async fn reversible_when_revert_json_present_and_not_undone() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = ActivityRepo::new(catalog.pools().clone());

        let mut tx = repo.begin().await.unwrap();
        let id = repo
            .append_in_tx(
                &mut tx,
                ActivityInput {
                    event_type: event_type::ASSET_TAGS_ADDED.to_string(),
                    actor: actor::USER.to_string(),
                    correlation_id: Some("corr-1".into()),
                    subject_type: Some("asset".into()),
                    subject_id: Some(1),
                    subject_key: None,
                    summary: None,
                    payload_json: "{}".into(),
                    revert_json: Some(r#"{"asset_ids":[1],"tag_id":2}"#.into()),
                },
            )
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let entry = repo.get(id).await.unwrap();
        assert!(entry.reversible);
        assert!(entry.revert_json.is_some());
        assert!(!repo.pool().is_closed());
    }

    #[tokio::test]
    async fn get_returns_not_found_for_missing_activity() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = ActivityRepo::new(catalog.pools().clone());
        let err = repo.get(999_999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn list_for_asset_filters_and_paginates() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = ActivityRepo::new(catalog.pools().clone());
        let mut tx = repo.begin().await.unwrap();
        for _ in 0..3 {
            repo.append_in_tx(
                &mut tx,
                ActivityInput {
                    event_type: event_type::ASSET_METADATA_CHANGED.to_string(),
                    actor: actor::USER.to_string(),
                    correlation_id: None,
                    subject_type: Some("asset".into()),
                    subject_id: Some(42),
                    subject_key: None,
                    summary: None,
                    payload_json: "{}".into(),
                    revert_json: None,
                },
            )
            .await
            .unwrap();
        }
        repo.append_in_tx(
            &mut tx,
            ActivityInput {
                event_type: event_type::ASSET_METADATA_CHANGED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(99),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: None,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let page = repo.list_for_asset(42, 0, 2).await.unwrap();
        assert_eq!(page.len(), 2);
        assert!(page.iter().all(|entry| entry.subject_id == Some(42)));
    }

    #[tokio::test]
    async fn mark_undone_sets_timestamp() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = ActivityRepo::new(catalog.pools().clone());
        let mut tx = repo.begin().await.unwrap();
        let id = repo
            .append_in_tx(
                &mut tx,
                ActivityInput {
                    event_type: event_type::ASSET_TAGS_ADDED.to_string(),
                    actor: actor::USER.to_string(),
                    correlation_id: None,
                    subject_type: Some("asset".into()),
                    subject_id: Some(1),
                    subject_key: None,
                    summary: None,
                    payload_json: "{}".into(),
                    revert_json: Some("{}".into()),
                },
            )
            .await
            .unwrap();
        repo.mark_undone(&mut tx, id, id).await.unwrap();
        tx.commit().await.unwrap();
        let entry = repo.get(id).await.unwrap();
        assert!(entry.undone_at.is_some());
        assert!(!entry.reversible);
    }

    #[tokio::test]
    async fn repo_methods_fail_when_pool_closed() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let repo = ActivityRepo::new(pools.clone());
        pools.close().await;
        assert!(repo.begin().await.is_err());
        assert!(repo.get(1).await.is_err());
        assert!(repo.list_for_asset(1, 0, 10).await.is_err());
    }
}

#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: i64,
    seq: i64,
    occurred_at: i64,
    event_type: String,
    actor: String,
    correlation_id: Option<String>,
    subject_type: Option<String>,
    subject_id: Option<i64>,
    subject_key: Option<String>,
    summary: Option<String>,
    payload_json: String,
    revert_json: Option<String>,
    undone_at: Option<i64>,
}

impl ActivityRow {
    fn into_entry(self) -> ActivityEntry {
        let reversible = self.revert_json.is_some() && self.undone_at.is_none();
        ActivityEntry {
            id: self.id,
            seq: self.seq,
            occurred_at: self.occurred_at,
            event_type: self.event_type,
            actor: self.actor,
            correlation_id: self.correlation_id,
            subject_type: self.subject_type,
            subject_id: self.subject_id,
            subject_key: self.subject_key,
            summary: self.summary,
            payload_json: self.payload_json,
            revert_json: self.revert_json,
            undone_at: self.undone_at,
            reversible,
        }
    }
}
