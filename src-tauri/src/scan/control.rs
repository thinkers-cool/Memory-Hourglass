use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ScanControl {
    pub paused: Arc<AtomicBool>,
    pub cancelled: Arc<AtomicBool>,
}

impl ScanControl {
    pub fn new(paused: Arc<AtomicBool>, cancelled: Arc<AtomicBool>) -> Self {
        Self { paused, cancelled }
    }

    pub fn noop() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancelled_flag(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }

    pub async fn wait_if_paused(&self) {
        while self.paused.load(Ordering::SeqCst) && !self.cancelled.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub(crate) fn take_env_cancel_flag(name: &str) -> bool {
    crate::scan::test_hooks::take_flag(name)
}

pub(crate) fn should_stop_inventory_batch(ctrl: &ScanControl) -> bool {
    ctrl.is_cancelled() || take_env_cancel_flag("MEMHG_TEST_CANCEL_AFTER_PAUSE")
}

pub(crate) fn should_stop_index_batch(ctrl: &ScanControl) -> bool {
    ctrl.is_cancelled()
        || take_env_cancel_flag("MEMHG_TEST_FORCE_INDEX_CANCEL")
        || take_env_cancel_flag("MEMHG_TEST_CANCEL_AFTER_PAUSE")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[tokio::test]
    async fn wait_if_paused_returns_when_not_paused() {
        let ctrl = ScanControl::noop();
        ctrl.wait_if_paused().await;
        assert!(!ctrl.is_cancelled());
    }

    #[tokio::test]
    async fn wait_if_paused_exits_when_cancelled() {
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        cancelled.store(true, Ordering::SeqCst);
        ctrl.wait_if_paused().await;
        assert!(ctrl.is_cancelled());
    }

    #[test]
    fn noop_control_is_not_cancelled() {
        let ctrl = ScanControl::noop();
        assert!(!ctrl.is_cancelled());
    }

    #[test]
    fn should_stop_inventory_batch_honors_cancel_flag_and_env() {
        crate::scan::test_hooks::with_env_test_lock(|| {
            crate::scan::test_hooks::reset_unlocked();
            let ctrl = ScanControl::noop();
            assert!(!should_stop_inventory_batch(&ctrl));
            crate::scan::test_hooks::set_flag("MEMHG_TEST_CANCEL_AFTER_PAUSE");
            assert!(should_stop_inventory_batch(&ctrl));
            assert!(!should_stop_inventory_batch(&ctrl));
            let cancelled = Arc::new(AtomicBool::new(true));
            let ctrl = ScanControl::new(Arc::new(AtomicBool::new(false)), cancelled);
            assert!(should_stop_inventory_batch(&ctrl));
        });
    }

    #[test]
    fn should_stop_index_batch_honors_all_cancel_sources() {
        crate::scan::test_hooks::with_env_test_lock(|| {
            crate::scan::test_hooks::reset_unlocked();
            let ctrl = ScanControl::noop();
            assert!(!should_stop_index_batch(&ctrl));
            crate::scan::test_hooks::set_flag("MEMHG_TEST_FORCE_INDEX_CANCEL");
            assert!(should_stop_index_batch(&ctrl));
            crate::scan::test_hooks::set_flag("MEMHG_TEST_CANCEL_AFTER_PAUSE");
            assert!(should_stop_index_batch(&ctrl));
        });
    }

    #[tokio::test]
    async fn wait_if_paused_waits_until_cancelled() {
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        let waiter = tokio::spawn(async move {
            ctrl.wait_if_paused().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        cancelled.store(true, Ordering::SeqCst);
        waiter.await.unwrap();
    }
}
