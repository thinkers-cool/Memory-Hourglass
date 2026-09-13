use crate::activity::models::ActivityInput;
use crate::activity::repo::ActivityRepo;
use crate::error::Result;
use crate::catalog::pools::CatalogPools;
use sqlx::Transaction;

pub struct ActivityRecorder {
    repo: ActivityRepo,
}

impl ActivityRecorder {
    pub fn new(pools: CatalogPools) -> Self {
        Self {
            repo: ActivityRepo::new(pools),
        }
    }

    pub fn repo(&self) -> &ActivityRepo {
        &self.repo
    }

    pub async fn append(&self, input: ActivityInput) -> Result<i64> {
        let mut tx = self.repo.begin().await?;
        let id = self.repo.append_in_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn append_in_tx(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        input: ActivityInput,
    ) -> Result<i64> {
        self.repo.append_in_tx(tx, input).await
    }
}

#[cfg(test)]
mod recorder_tests {
    use super::*;
    use crate::activity::models::ActivityInput;
    use crate::catalog::Catalog;
    use tempfile::tempdir;

    #[tokio::test]
    async fn append_in_tx_commits_through_repo() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let recorder = ActivityRecorder::new(catalog.pools().clone());
        let mut tx = recorder.repo().begin().await.unwrap();
        let id = recorder
            .append_in_tx(
                &mut tx,
                ActivityInput {
                    event_type: "test.event".into(),
                    actor: "user".into(),
                    correlation_id: None,
                    subject_type: None,
                    subject_id: None,
                    subject_key: None,
                    summary: None,
                    payload_json: "{}".into(),
                    revert_json: None,
                },
            )
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn append_commits_transaction() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let recorder = ActivityRecorder::new(catalog.pools().clone());
        let id = recorder
            .append(ActivityInput {
                event_type: "test.append".into(),
                actor: "user".into(),
                correlation_id: None,
                subject_type: None,
                subject_id: None,
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: None,
            })
            .await
            .unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn append_fails_when_pool_closed() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let recorder = ActivityRecorder::new(pools.clone());
        pools.close().await;
        let err = recorder
            .append(ActivityInput {
                event_type: "test.closed".into(),
                actor: "user".into(),
                correlation_id: None,
                subject_type: None,
                subject_id: None,
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: None,
            })
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
