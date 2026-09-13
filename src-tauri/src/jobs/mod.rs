use crate::error::{AppError, Result};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const MAX_PARALLEL_SCANS: usize = 2;

struct ExclusiveSlot {
    name: String,
    cancel: Arc<AtomicBool>,
}

struct JobPoolInner {
    scan_jobs: Mutex<HashMap<i64, Arc<AtomicBool>>>,
    scan_pending: Mutex<VecDeque<i64>>,
    exclusive: Mutex<Option<ExclusiveSlot>>,
}

pub struct JobPool {
    inner: Arc<JobPoolInner>,
}

impl Clone for JobPool {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for JobPool {
    fn default() -> Self {
        Self::new()
    }
}

impl JobPool {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(JobPoolInner {
                scan_jobs: Mutex::new(HashMap::new()),
                scan_pending: Mutex::new(VecDeque::new()),
                exclusive: Mutex::new(None),
            }),
        }
    }

    pub async fn is_scan_running(&self, root_id: i64) -> bool {
        self.inner.scan_jobs.lock().await.contains_key(&root_id)
    }

    pub async fn active_scan_count(&self) -> usize {
        self.inner.scan_jobs.lock().await.len()
    }

    pub async fn try_start_scan(&self, root_id: i64) -> Result<Arc<AtomicBool>> {
        {
            let scans = self.inner.scan_jobs.lock().await;
            if scans.contains_key(&root_id) {
                return Err(AppError::Job("scan already running for this root".into()));
            }
            if scans.len() >= MAX_PARALLEL_SCANS {
                return Err(AppError::Job("scan pool full".into()));
            }
        }
        if self.inner.exclusive.lock().await.is_some() {
            return Err(AppError::Job("exclusive job is running".into()));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.inner
            .scan_jobs
            .lock()
            .await
            .insert(root_id, cancel.clone());
        Ok(cancel)
    }

    pub async fn finish_scan(&self, root_id: i64) {
        self.inner.scan_jobs.lock().await.remove(&root_id);
    }

    pub async fn enqueue_scan(&self, root_id: i64) {
        let mut pending = self.inner.scan_pending.lock().await;
        if pending.contains(&root_id) {
            return;
        }
        if self.inner.scan_jobs.lock().await.contains_key(&root_id) {
            return;
        }
        pending.push_back(root_id);
    }

    pub async fn dequeue_pending_scan(&self) -> Option<i64> {
        let mut pending = self.inner.scan_pending.lock().await;
        while let Some(root_id) = pending.pop_front() {
            if self.inner.scan_jobs.lock().await.contains_key(&root_id) {
                continue;
            }
            return Some(root_id);
        }
        None
    }

    pub async fn pending_scan_roots(&self) -> Vec<i64> {
        self.inner.scan_pending.lock().await.iter().copied().collect()
    }

    pub async fn request_cancel_scan(&self, root_id: i64) {
        if let Some(cancel) = self.inner.scan_jobs.lock().await.get(&root_id) {
            cancel.store(true, Ordering::SeqCst);
        }
    }

    pub async fn cancel_and_clear_root(&self, root_id: i64) {
        self.request_cancel_scan(root_id).await;
        self.inner
            .scan_pending
            .lock()
            .await
            .retain(|id| *id != root_id);
    }

    pub async fn wait_scan_stopped(&self, root_id: i64, max_wait: std::time::Duration) {
        let deadline = tokio::time::Instant::now() + max_wait;
        while self.is_scan_running(root_id).await && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    pub async fn request_cancel_all_scans(&self) {
        for cancel in self.inner.scan_jobs.lock().await.values() {
            cancel.store(true, Ordering::SeqCst);
        }
    }

    pub async fn try_start_exclusive(&self, job_name: &str) -> Result<Arc<AtomicBool>> {
        let mut slot = self.inner.exclusive.lock().await;
        if slot.is_some() {
            return Err(AppError::Job("another job is running".into()));
        }
        if !self.inner.scan_jobs.lock().await.is_empty() {
            return Err(AppError::Job("scan jobs are running".into()));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *slot = Some(ExclusiveSlot {
            name: job_name.to_string(),
            cancel: cancel.clone(),
        });
        Ok(cancel)
    }

    pub async fn finish_exclusive(&self) {
        *self.inner.exclusive.lock().await = None;
    }

    pub async fn request_cancel_exclusive(&self) {
        if let Some(slot) = self.inner.exclusive.lock().await.as_ref() {
            slot.cancel.store(true, Ordering::SeqCst);
        }
    }

    pub async fn is_idle(&self) -> bool {
        self.inner.scan_jobs.lock().await.is_empty()
            && self.inner.exclusive.lock().await.is_none()
    }

    pub async fn current_exclusive(&self) -> Option<String> {
        self.inner
            .exclusive
            .lock()
            .await
            .as_ref()
            .map(|slot| slot.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parallel_scans_up_to_limit() {
        let pool = JobPool::new();
        pool.try_start_scan(1).await.unwrap();
        pool.try_start_scan(2).await.unwrap();
        pool.try_start_scan(1).await.unwrap_err();
        assert_eq!(pool.active_scan_count().await, 2);
        pool.finish_scan(1).await;
        pool.try_start_scan(3).await.unwrap();
    }

    #[tokio::test]
    async fn scan_pool_full_rejects_new_root() {
        let pool = JobPool::new();
        for id in 1..=MAX_PARALLEL_SCANS as i64 {
            pool.try_start_scan(id).await.unwrap();
        }
        pool.try_start_scan(99).await.unwrap_err();
    }

    #[tokio::test]
    async fn exclusive_blocks_while_scans_active() {
        let pool = JobPool::new();
        pool.try_start_scan(1).await.unwrap();
        pool.try_start_exclusive("export").await.unwrap_err();
        pool.finish_scan(1).await;
        pool.try_start_exclusive("export").await.unwrap();
    }

    #[tokio::test]
    async fn scan_blocks_exclusive_job() {
        let pool = JobPool::new();
        pool.try_start_exclusive("rebuild").await.unwrap();
        pool.try_start_scan(1).await.unwrap_err();
    }

    #[tokio::test]
    async fn enqueue_and_dequeue_pending() {
        let pool = JobPool::new();
        pool.enqueue_scan(5).await;
        pool.enqueue_scan(5).await;
        assert_eq!(pool.dequeue_pending_scan().await, Some(5));
        assert!(pool.dequeue_pending_scan().await.is_none());
    }

    #[tokio::test]
    async fn cancel_scan_sets_flag() {
        let pool = JobPool::new();
        let cancel = pool.try_start_scan(1).await.unwrap();
        pool.request_cancel_scan(1).await;
        assert!(cancel.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancel_and_clear_root_cancels_running_and_drops_pending() {
        let pool = JobPool::new();
        let cancel = pool.try_start_scan(7).await.unwrap();
        pool.enqueue_scan(7).await;
        pool.enqueue_scan(8).await;
        pool.cancel_and_clear_root(7).await;
        assert!(cancel.load(Ordering::SeqCst));
        assert_eq!(pool.pending_scan_roots().await, vec![8]);
    }
}
