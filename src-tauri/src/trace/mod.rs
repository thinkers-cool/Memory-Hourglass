use crate::error::Result;
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg_attr(coverage_nightly, coverage(off))]
pub fn init(log_dir: &Path) -> Result<WorkerGuard> {
    std::fs::create_dir_all(log_dir)?;

    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "memhg.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "memhg=debug,tauri=warn,sqlx=warn".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    Ok(guard)
}

pub fn new_correlation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn asset_subject_key(root_id: i64, rel_path: &str) -> String {
    format!("{}:{}", root_id, rel_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlation_helpers_work() {
        let correlation_id = new_correlation_id();
        assert!(!correlation_id.is_empty());
        assert_eq!(asset_subject_key(3, "photos/a.jpg"), "3:photos/a.jpg");
    }
}
