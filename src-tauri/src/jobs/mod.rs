use crate::error::{AppError, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct JobQueue {
    busy: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    current_job: Arc<Mutex<Option<String>>>,
}

impl Clone for JobQueue {
    fn clone(&self) -> Self {
        Self {
            busy: self.busy.clone(),
            cancel: self.cancel.clone(),
            current_job: self.current_job.clone(),
        }
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            busy: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
            current_job: Arc::new(Mutex::new(None)),
        }
    }

    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    pub async fn try_start(&self, job_name: &str) -> Result<()> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(AppError::Job("another job is running".into()));
        }
        self.cancel.store(false, Ordering::SeqCst);
        *self.current_job.lock().await = Some(job_name.to_string());
        Ok(())
    }

    pub async fn finish(&self) {
        *self.current_job.lock().await = None;
        self.cancel.store(false, Ordering::SeqCst);
        self.busy.store(false, Ordering::SeqCst);
    }

    pub async fn current(&self) -> Option<String> {
        self.current_job.lock().await.clone()
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_queue_is_idle() {
        let queue = JobQueue::default();
        assert!(!queue.is_cancelled());
    }

    #[tokio::test]
    async fn rejects_concurrent_jobs() {
        let queue = JobQueue::new();
        queue.try_start("scan").await.unwrap();
        let err = queue.try_start("export").await.unwrap_err();
        assert!(err.to_string().contains("running"));
        queue.finish().await;
        queue.try_start("export").await.unwrap();
    }

    #[tokio::test]
    async fn cancel_flag_works() {
        let queue = JobQueue::new();
        assert!(!queue.is_cancelled());
        queue.request_cancel();
        assert!(queue.is_cancelled());
    }

    #[tokio::test]
    async fn tracks_current_job_and_clears_on_finish() {
        let queue = JobQueue::new();
        queue.try_start("export").await.unwrap();
        assert_eq!(queue.current().await.as_deref(), Some("export"));
        queue.finish().await;
        assert!(queue.current().await.is_none());
        assert!(!queue.is_cancelled());
    }
}
