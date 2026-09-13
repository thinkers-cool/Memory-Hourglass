use crate::error::{AppError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::time::Duration;

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const SQLITE_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_RETRY_ATTEMPTS: u32 = 6;
const WRITE_RETRY_BASE_MS: u64 = 25;

#[derive(Clone)]
pub struct CatalogPools {
    read: SqlitePool,
    write: SqlitePool,
}

pub fn app_error_is_sqlite_busy(err: &AppError) -> bool {
    match err {
        AppError::Sqlx(sqlx_err) => sqlite_error_is_busy(sqlx_err),
        _ => false,
    }
}

pub fn sqlite_error_is_busy(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => {
            db.code().as_deref() == Some("5")
                || db.message().contains("database is locked")
                || db.message().contains("SQLITE_BUSY")
        }
        _ => false,
    }
}

pub async fn write_retry<T, F, Fut>(_write: &SqlitePool, mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0u32;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) if app_error_is_sqlite_busy(&err) && attempt + 1 < WRITE_RETRY_ATTEMPTS => {
                attempt += 1;
                let delay = WRITE_RETRY_BASE_MS * attempt as u64;
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            Err(err) => return Err(err),
        }
    }
}

fn sqlite_connect_options(db_path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
}

impl CatalogPools {
    pub async fn open(db_path: &Path) -> Result<Self> {
        let options = sqlite_connect_options(db_path);

        let write = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(SQLITE_ACQUIRE_TIMEOUT)
            .connect_with(options.clone())
            .await?;

        sqlx::migrate!("./migrations").run(&write).await?;

        let read = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(SQLITE_ACQUIRE_TIMEOUT)
            .connect_with(options.read_only(true))
            .await?;

        Ok(Self { read, write })
    }

    pub fn from_single(pool: SqlitePool) -> Self {
        Self {
            read: pool.clone(),
            write: pool,
        }
    }

    pub fn read(&self) -> &SqlitePool {
        &self.read
    }

    pub fn write(&self) -> &SqlitePool {
        &self.write
    }

    pub async fn close(&self) {
        self.read.close().await;
        self.write.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::{app_error_is_sqlite_busy, sqlite_error_is_busy, write_retry, CatalogPools};
    use crate::error::AppError;
    use sqlx::error::DatabaseError;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::borrow::Cow;
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };
    use tempfile::tempdir;

    #[derive(Debug)]
    struct BusyDatabaseError;

    impl std::fmt::Display for BusyDatabaseError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.message())
        }
    }

    impl std::error::Error for BusyDatabaseError {}

    impl DatabaseError for BusyDatabaseError {
        fn message(&self) -> &str {
            "database is locked"
        }

        fn code(&self) -> Option<Cow<'_, str>> {
            Some(Cow::Borrowed("5"))
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }

        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
    }

    fn busy_sqlx_error() -> sqlx::Error {
        sqlx::Error::Database(Box::new(BusyDatabaseError))
    }

    #[test]
    fn sqlite_error_is_busy_detects_locked_database() {
        assert!(sqlite_error_is_busy(&busy_sqlx_error()));
        assert!(!sqlite_error_is_busy(&sqlx::Error::RowNotFound));
        assert!(!app_error_is_sqlite_busy(&AppError::Export("x".into())));
        assert!(app_error_is_sqlite_busy(&AppError::Sqlx(busy_sqlx_error())));
    }

    #[tokio::test]
    async fn write_retry_succeeds_after_busy_errors() {
        let dir = tempdir().unwrap();
        let pool = SqlitePoolOptions::new()
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(dir.path().join("retry.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let attempts = Arc::new(AtomicU32::new(0));
        let value = write_retry(&pool, {
            let attempts = attempts.clone();
            move || {
                let attempts = attempts.clone();
                async move {
                    let next = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if next < 3 {
                        return Err(AppError::Sqlx(busy_sqlx_error()));
                    }
                    Ok(7)
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(value, 7);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn catalog_pools_open_read_write_and_close() {
        let dir = tempdir().unwrap();
        let pools = CatalogPools::open(&dir.path().join("catalog.db")).await.unwrap();
        assert!(pools.read().size() > 0);
        assert!(pools.write().size() > 0);
        let single = CatalogPools::from_single(pools.write().clone());
        assert!(single.read().size() > 0);
        pools.close().await;
    }
}
